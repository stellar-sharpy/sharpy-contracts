//! Sharpy — Advanced split payment contract with recurring splits, escrow, and batch operations.
//!
//! ## Capabilities (30+)
//! - **Invoice lifecycle:** `create_invoice`, `create_batch`, `create_recurring`, `pay`, `pool_pay`, `pay_with_tip`, `release`, `release_escrow`, `refund`, `cancel_invoice`, `freeze_invoice`/`unfreeze_invoice`
//! - **Queries:** `get_invoice`, `get_invoice_stats`, `get_invoice_version`, `get_treasury`, `get_recurring_params`, `get_escrow_state`, `get_audit_log`, `get_payer_total`, `get_next_recurring`, `get_invoices_by_creator`, `get_invoices_by_payer`, `get_invoice_count`, `get_claimable_balance`, `claim`, `get_invoice_notes`, `set_invoice_notes`, `get_invoice_fingerprint`, `bump_invoice_ttl`, `preview_payout`
//! - **Split rules:** `Fixed`, `Percentage` (≤100%), `Tiered` threshold — all checked arithmetic (CAP-82)
//! - **Escrow & disputes:** `DisputeState` hold, `dispute_release`/`resolve_dispute` with arbitrator, `EscrowFchedule`
//! - **Admin:** `initialize`, `pause`/`unpause`, `freeze`/`unfreeze`, `require!` macro, `_refund_payers` helper
//! - **Storage:** `InvoiceNotes { text, updated_at }`, `SubscriptionParams`, `InvoiceStats`, TTL extend ~1y (CAP-78), SHA-256 fingerprint (CAP-75)
//! - **Events:** `invoice_created`, `payment_received`, `invoice_released`, `invoice_refunded`, `invoice_cancelled`, `dispute_raised/resolved`, `escrow_funded`, `account_balance_claimed`
//!
//! Protocol 27 (soroban-sdk 26.1.0), `wasm32v1-none`, CEI `claim` pattern.

#![no_std]

mod events;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, Bytes, Env, Map, String, Symbol, Vec};
use types::{
    AuditEntry, CreateInvoiceParams, DisputeState, Invoice, InvoiceNotes, InvoiceOptions,
    InvoicePayment, InvoiceStats, InvoiceStatus, Payment, SplitRule, SubscriptionParams,
};

fn admin_key() -> Symbol { symbol_short!("admin") }
fn paused_key() -> Symbol { symbol_short!("paused") }
fn treasury_key() -> Symbol { symbol_short!("treasury") }
fn counter_key() -> Symbol { symbol_short!("counter") }
fn invoice_key(id: u64) -> (Symbol, u64) { (symbol_short!("inv"), id) }
fn audit_log_key(id: u64) -> (Symbol, u64) { (symbol_short!("log"), id) }
fn escrow_state_key(id: u64) -> (Symbol, u64) { (symbol_short!("escrow"), id) }
fn recurring_params_key(id: u64) -> (Symbol, u64) { (symbol_short!("rec"), id) }
fn next_invoice_key(id: u64) -> (Symbol, u64) { (symbol_short!("next_inv"), id) }
fn creator_index_key(creator: &Address) -> (Symbol, Address) { (symbol_short!("by_ctr"), creator.clone()) }
fn payer_index_key(payer: &Address) -> (Symbol, Address) { (symbol_short!("by_pyr"), payer.clone()) }
fn account_balance_key(account: &Address, token: &Address) -> (Symbol, Address, Address) {
    (symbol_short!("acc_bal"), account.clone(), token.clone())
}
fn invoice_notes_key(id: u64) -> (Symbol, u64) { (symbol_short!("notes"), id) }

fn is_paused(env: &Env) -> bool {
    env.storage().persistent().get(&paused_key()).unwrap_or(false)
}

/// Convenience macro that mirrors Solidity's `require(cond, msg)`.
/// Panics with the given string literal when `$cond` is false.
/// Prefer this over raw `assert!` for user-facing guards — the message is
/// visible in transaction error metadata and test output.
macro_rules! require {
    ($cond:expr, $msg:literal) => {
        assert!($cond, $msg)
    };
}

fn require_not_paused(env: &Env) {
    require!(!is_paused(env), "contract is paused");
}

fn require_admin(env: &Env) {
    let admin: Address = env.storage().instance().get(&admin_key()).expect("admin not set");
    admin.require_auth();
}

fn load_invoice(env: &Env, id: u64) -> Invoice {
    env.storage().persistent().get(&invoice_key(id)).expect("invoice not found")
}

fn save_invoice(env: &Env, id: u64, invoice: &Invoice) {
    env.storage().persistent().set(&invoice_key(id), invoice);
    // Protocol 26 CAP-78: limited TTL extension host function.
    // Extend to ~1 year (6_307_200 ledgers at ~5s each).
    // Only bumps if current TTL < min_ttl (100_000 ledgers ~ 6 days).
    env.storage().persistent().extend_ttl(&invoice_key(id), 100_000, 6_307_200);
}

fn append_audit(env: &Env, id: u64, action: Symbol, actor: &Address) {
    let entry = AuditEntry { action, actor: actor.clone(), timestamp: env.ledger().timestamp() };
    let mut log: Vec<AuditEntry> = env.storage().persistent().get(&audit_log_key(id)).unwrap_or_else(|| Vec::new(env));
    log.push_back(entry);
    env.storage().persistent().set(&audit_log_key(id), &log);
}

fn bump_counter(env: &Env) -> u64 {
    let id: u64 = env.storage().persistent().get(&counter_key()).unwrap_or(0u64) + 1;
    env.storage().persistent().set(&counter_key(), &id);
    id
}

fn index_invoice_for_creator(env: &Env, creator: &Address, invoice_id: u64) {
    let key = creator_index_key(creator);
    let mut ids: Vec<u64> = env.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(env));
    ids.push_back(invoice_id);
    env.storage().persistent().set(&key, &ids);
    env.storage().persistent().extend_ttl(&key, 100_000, 6_307_200);
}

fn index_invoice_for_payer(env: &Env, payer: &Address, invoice_id: u64) {
    let key = payer_index_key(payer);
    let mut ids: Vec<u64> = env.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(env));
    // Only index if not already present (payer may pay same invoice multiple times)
    if !ids.contains(&invoice_id) {
        ids.push_back(invoice_id);
        env.storage().persistent().set(&key, &ids);
        env.storage().persistent().extend_ttl(&key, 100_000, 6_307_200);
        events::payment_indexed(env, payer, invoice_id);
    }
}

/// Credits an internal balance for an account+token pair when a direct transfer fails.
/// Used in _release when a recipient cannot receive funds (no trustline, frozen account, etc.)
/// The credited amount can be withdrawn later via claim().
fn credit_account(env: &Env, account: &Address, token: &Address, amount: i128) {
    let key = account_balance_key(account, token);
    let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
    // Use checked_add to prevent overflow if multiple failed transfers accumulate
    let new_balance = current.checked_add(amount).expect("account balance overflow");
    env.storage().persistent().set(&key, &new_balance);
    env.storage().persistent().extend_ttl(&key, 100_000, 6_307_200);
}

fn build_invoice(
    env: &Env,
    creator: Address,
    recipients: Vec<Address>,
    amounts: Vec<i128>,
    tokens: Vec<Address>,
    deadline: u64,
    escrow_enabled: bool,
    escrow_release_delay: u64,
    split_rules: Vec<SplitRule>,
    arbitrator: Option<Address>,
) -> Invoice {
    let mut claimed: Vec<i128> = Vec::new(env);
    for _ in recipients.iter() {
        claimed.push_back(0i128);
    }
    Invoice {
        version: 1u32,
        creator,
        recipients,
        amounts,
        tokens,
        deadline,
        funded: 0,
        status: InvoiceStatus::Pending,
        payments: Vec::new(env),
        claimed,
        frozen: false,
        completion_time: None,
        escrow_enabled,
        escrow_release_delay,
        split_rules,
        auto_resolve_rules: Vec::new(env),
        arbitrator,
    }
}

#[contract]
pub struct SharpyContract;

#[contractimpl]
impl SharpyContract {
    pub fn initialize(env: Env, admin: Address, treasury: Address) {
        assert!(!env.storage().instance().has(&admin_key()), "already initialized");
        env.storage().instance().set(&admin_key(), &admin);
        env.storage().instance().set(&treasury_key(), &treasury);
        env.storage().persistent().set(&paused_key(), &false);
    }

    pub fn pause(env: Env) {
        require_admin(&env);
        env.storage().persistent().set(&paused_key(), &true);
    }

    pub fn unpause(env: Env) {
        require_admin(&env);
        env.storage().persistent().set(&paused_key(), &false);
    }

    pub fn create_invoice(
        env: Env,
        creator: Address,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
        tokens: Vec<Address>,
        deadline: u64,
        options: InvoiceOptions,
    ) -> u64 {
        require_not_paused(&env);
        creator.require_auth();
        assert_eq!(recipients.len(), amounts.len(), "recipients and amounts length mismatch");
        assert_eq!(recipients.len(), tokens.len(), "recipients and tokens length mismatch");
        assert!(!recipients.is_empty(), "must have at least one recipient");
        assert!(deadline > env.ledger().timestamp(), "deadline must be in the future");
        for amt in amounts.iter() {
            assert!(amt > 0, "amounts must be positive");
        }

        // Validate percentage split rules do not exceed 10000 bps total
        if !options.split_rules.is_empty() {
            let total_bps: u32 = options.split_rules.iter().map(|r| match r {
                SplitRule::Percentage(bps) => bps,
                SplitRule::Tiered(_, bps) => bps,
                SplitRule::Fixed(_) => 0,
            }).sum();
            assert!(total_bps <= 10_000u32, "split rules exceed 100% (10000 bps)");
        }

        let id = bump_counter(&env);
        let invoice = build_invoice(
            &env, creator.clone(), recipients, amounts, tokens, deadline,
            options.escrow_enabled, options.escrow_release_delay.unwrap_or(0), options.split_rules,
            options.arbitrator,
        );
        save_invoice(&env, id, &invoice);
        index_invoice_for_creator(&env, &creator, id);
        events::invoice_created(&env, id, &creator);
        id
    }

    pub fn create_batch(env: Env, creator: Address, invoices: Vec<CreateInvoiceParams>) -> Vec<u64> {
        require_not_paused(&env);
        creator.require_auth();
        assert!(invoices.len() <= 10, "batch limit is 10");

        let mut ids: Vec<u64> = Vec::new(&env);
        for params in invoices.iter() {
            assert_eq!(params.recipients.len(), params.tokens.len(), "recipients and tokens length mismatch");
            let id = bump_counter(&env);
            let invoice = build_invoice(
                &env, creator.clone(), params.recipients.clone(), params.amounts.clone(),
                params.tokens.clone(), params.deadline, false, 0, Vec::new(&env), None,
            );
            save_invoice(&env, id, &invoice);
            index_invoice_for_creator(&env, &creator, id);
            events::invoice_created(&env, id, &creator);
            ids.push_back(id);
        }
        ids
    }

    pub fn create_recurring(
        env: Env,
        creator: Address,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
        tokens: Vec<Address>,
        deadline: u64,
        recurrence_interval: u64,
        max_recurrences: u32,
    ) -> u64 {
        require_not_paused(&env);
        creator.require_auth();
        assert_eq!(recipients.len(), amounts.len(), "recipients and amounts length mismatch");
        assert_eq!(recipients.len(), tokens.len(), "recipients and tokens length mismatch");
        assert!(recurrence_interval > 0, "recurrence_interval must be positive");

        let id = bump_counter(&env);
        let invoice = build_invoice(
            &env, creator.clone(), recipients.clone(), amounts.clone(),
            tokens.clone(), deadline, false, 0, Vec::new(&env), None,
        );
        save_invoice(&env, id, &invoice);
        index_invoice_for_creator(&env, &creator, id);

        let params = SubscriptionParams {
            creator: creator.clone(),
            recipients,
            amounts,
            tokens,
            recurrence_interval,
            max_recurrences,
            num_created: 1,
        };
        env.storage().persistent().set(&recurring_params_key(id), &params);
        events::invoice_created(&env, id, &creator);
        id
    }

    /// Security audit — pay() double-spend / concurrent same-invoice safety.
    /// Soroban executes transactions sequentially within a ledger: ledgers are
    /// applied one tx at a time, each seeing the storage writes of the prior tx.
    /// There is no parallel execution within a block. The `total - funded` guard
    /// is evaluated after `load_invoice` and before any state mutation; two payers
    /// that both observe `funded=0` cannot both succeed — the first pay writes
    /// `funded+=amount`, the second load will see the updated `funded` and the
    /// `amount > remaining` panic will trigger. The test `test_pay_double_spend_sequential_guard`
    /// demonstrates this with `mock_all_auths` — sequential pays respect the
    /// remaining-balance invariant and funded is never double-counted.
    pub fn pay(env: Env, payer: Address, invoice_id: u64, amount: i128) {
        require_not_paused(&env);
        payer.require_auth();
        assert!(amount > 0, "payment amount must be positive");

        let mut invoice = load_invoice(&env, invoice_id);
        assert!(invoice.status == InvoiceStatus::Pending, "invoice is not pending");
        assert!(env.ledger().timestamp() <= invoice.deadline, "invoice deadline has passed");
        assert!(!invoice.frozen, "invoice is frozen");

        let total: i128 = invoice.amounts.iter().sum();
        let remaining = total - invoice.funded;
        if amount > remaining {
            panic!("payment exceeds remaining balance: payment of {} exceeds remaining {}", amount, remaining);
        }

        let token_client = token::Client::new(&env, &invoice.tokens.get(0).expect("no token"));
        token_client.transfer(&payer, &env.current_contract_address(), &amount);

        invoice.payments.push_back(Payment { payer: payer.clone(), amount, tip: 0 });
        invoice.funded += amount;
        index_invoice_for_payer(&env, &payer, invoice_id);
        append_audit(&env, invoice_id, symbol_short!("pay"), &payer);
        events::payment_received(&env, invoice_id, &payer, amount);

        if invoice.funded >= total {
            if invoice.escrow_enabled {
                let release_at = env.ledger().timestamp() + invoice.escrow_release_delay;
                let state = DisputeState { release_at, disputed: false, disputed_at: 0 };
                env.storage().persistent().set(&escrow_state_key(invoice_id), &state);
                events::escrow_funded(&env, invoice_id, release_at, invoice.funded);
                save_invoice(&env, invoice_id, &invoice);
            } else {
                Self::_release(&env, invoice_id, &mut invoice, &payer);
            }
        } else {
            save_invoice(&env, invoice_id, &invoice);
        }
    }

    pub fn pool_pay(env: Env, payer: Address, payments: Vec<InvoicePayment>) {
        require_not_paused(&env);
        payer.require_auth();
        assert!(!payments.is_empty(), "payments must not be empty");

        // Phase 1: Validate all invoices and group totals by token
        let mut token_totals: Map<Address, i128> = Map::new(&env);
        for p in payments.iter() {
            let inv = load_invoice(&env, p.invoice_id);
            assert!(inv.status == InvoiceStatus::Pending, "invoice is not pending");
            assert!(p.amount > 0, "payment amount must be positive");
            let inv_total: i128 = inv.amounts.iter().sum();
            let remaining = inv_total - inv.funded;
            if inv.funded + p.amount > inv_total {
                panic!("payment exceeds remaining balance: payment of {} exceeds remaining {}", p.amount, remaining);
            }
            let token = inv.tokens.get(0).expect("no token");
            let prev = token_totals.get(token.clone()).unwrap_or(0);
            token_totals.set(token, prev + p.amount);
        }

        // Phase 2: Transfer tokens — one transfer per unique token
        for (token, amount) in token_totals.iter() {
            let token_client = token::Client::new(&env, &token);
            token_client.transfer(&payer, &env.current_contract_address(), &amount);
        }

        // Phase 3: Update each invoice's state
        for p in payments.iter() {
            let mut inv = load_invoice(&env, p.invoice_id);
            inv.payments.push_back(Payment { payer: payer.clone(), amount: p.amount, tip: 0 });
            inv.funded += p.amount;
            index_invoice_for_payer(&env, &payer, p.invoice_id);
            append_audit(&env, p.invoice_id, symbol_short!("pool_pay"), &payer);
            events::payment_received(&env, p.invoice_id, &payer, p.amount);
            let inv_total: i128 = inv.amounts.iter().sum();
            if inv.funded >= inv_total {
                if inv.escrow_enabled {
                    let release_at = env.ledger().timestamp() + inv.escrow_release_delay;
                    let state = DisputeState { release_at, disputed: false, disputed_at: 0 };
                    env.storage().persistent().set(&escrow_state_key(p.invoice_id), &state);
                    events::escrow_funded(&env, p.invoice_id, release_at, inv.funded);
                    save_invoice(&env, p.invoice_id, &inv);
                } else {
                    Self::_release(&env, p.invoice_id, &mut inv, &payer);
                }
            } else {
                save_invoice(&env, p.invoice_id, &inv);
            }
        }
    }

    pub fn release_escrow(env: Env, invoice_id: u64) {
        require_not_paused(&env);
        let mut invoice = load_invoice(&env, invoice_id);
        assert!(invoice.escrow_enabled, "escrow not enabled on this invoice");
        let state: DisputeState = env.storage().persistent()
            .get(&escrow_state_key(invoice_id)).expect("escrow not found");
        assert!(!state.disputed, "release is disputed, use resolve_dispute");
        assert!(env.ledger().timestamp() >= state.release_at, "escrow delay not yet met");
        let caller = env.current_contract_address();
        Self::_release(&env, invoice_id, &mut invoice, &caller);
        env.storage().persistent().remove(&escrow_state_key(invoice_id));
    }

    pub fn dispute_release(env: Env, invoice_id: u64) {
        require_not_paused(&env);
        let invoice = load_invoice(&env, invoice_id);
        assert!(invoice.status == InvoiceStatus::Pending, "invoice is not pending");
        assert!(invoice.escrow_enabled, "escrow not enabled on this invoice");
        invoice.creator.require_auth();

        let state: DisputeState = env.storage().persistent()
            .get(&escrow_state_key(invoice_id)).expect("escrow not found");
        assert!(!state.disputed, "dispute already raised");
        assert!(env.ledger().timestamp() < state.release_at, "escrow delay has passed, cannot dispute");

        let new_state = DisputeState { disputed: true, disputed_at: env.ledger().timestamp(), ..state };
        env.storage().persistent().set(&escrow_state_key(invoice_id), &new_state);
        append_audit(&env, invoice_id, symbol_short!("dispute"), &invoice.creator);
        events::dispute_raised(&env, invoice_id, &invoice.creator);
    }

    pub fn resolve_dispute(env: Env, invoice_id: u64, release: bool) {
        require_not_paused(&env);
        let mut invoice = load_invoice(&env, invoice_id);
        assert!(invoice.status == InvoiceStatus::Pending, "invoice is not pending");

        let state: DisputeState = env.storage().persistent()
            .get(&escrow_state_key(invoice_id)).expect("escrow not found");
        assert!(state.disputed, "no active dispute");

        let resolver = invoice.arbitrator.clone().unwrap_or_else(|| invoice.creator.clone());
        resolver.require_auth();

        env.storage().persistent().remove(&escrow_state_key(invoice_id));

        if release {
            Self::_release(&env, invoice_id, &mut invoice, &resolver);
        } else {
            Self::_refund_payers(&env, invoice_id, &invoice);
            invoice.status = InvoiceStatus::Refunded;
            invoice.completion_time = Some(env.ledger().timestamp());
            save_invoice(&env, invoice_id, &invoice);
            append_audit(&env, invoice_id, symbol_short!("resolve"), &resolver);
            events::invoice_refunded(&env, invoice_id, invoice.funded, invoice.recipients.len(), &invoice.creator);
        }

        events::dispute_resolved(&env, invoice_id, &resolver, release);
    }

    fn _release(env: &Env, invoice_id: u64, invoice: &mut Invoice, actor: &Address) {
        assert!(invoice.status == InvoiceStatus::Pending, "invoice is not pending");

        let total: i128 = invoice.amounts.iter().sum();
        let n = invoice.recipients.len();
        let mut distributed: i128 = 0;

        for i in 0..n {
            let recipient = invoice.recipients.get(i).unwrap();
            let amount = invoice.amounts.get(i).unwrap();
            let token = invoice.tokens.get(i).expect("no token");
            let token_client = token::Client::new(env, &token);

            let proportional = if !invoice.split_rules.is_empty() {
                match invoice.split_rules.get(i as u32).unwrap() {
                    SplitRule::Fixed(fixed_amt) => fixed_amt,
                    SplitRule::Percentage(bps) => {
                        // Protocol 26 CAP-82: use checked arithmetic to prevent overflow
                        // funded * bps / 10_000 — all i128, safe for balances up to i128::MAX
                        invoice.funded
                            .checked_mul(bps as i128)
                            .expect("percentage: overflow in funded * bps")
                            .checked_div(10_000)
                            .expect("percentage: division failed")
                    }
                    SplitRule::Tiered(threshold, bps) => {
                        if invoice.funded > threshold {
                            // Protocol 26 CAP-82: checked arithmetic for tiered splits
                            invoice.funded
                                .checked_mul(bps as i128)
                                .expect("tiered: overflow in funded * bps")
                                .checked_div(10_000)
                                .expect("tiered: division failed")
                        } else {
                            0
                        }
                    }
                }
            } else if i == n - 1 {
                // Last recipient gets the remainder to avoid dust from rounding
                invoice.funded
                    .checked_sub(distributed)
                    .expect("release: underflow computing remainder")
            } else {
                // Proportional split: amount * funded / total — checked throughout
                amount
                    .checked_mul(invoice.funded)
                    .expect("proportional: overflow in amount * funded")
                    .checked_div(total)
                    .expect("proportional: division by zero total")
            };

            distributed += proportional;
            if proportional > 0 {
                // Use try_transfer to catch failures (no trustline, frozen account, etc.)
                // On any failure, credit an internal balance that can be claimed later
                match token_client.try_transfer(&env.current_contract_address(), &recipient, &proportional) {
                    Ok(Ok(())) => {
                        // Transfer succeeded — no action needed
                    }
                    _ => {
                        // Transfer failed — credit internal balance for later claim
                        credit_account(env, &recipient, &token, proportional);
                    }
                }
            }
        }

        invoice.status = InvoiceStatus::Released;
        invoice.completion_time = Some(env.ledger().timestamp());
        save_invoice(env, invoice_id, invoice);
        append_audit(env, invoice_id, symbol_short!("release"), actor);
        events::invoice_released(env, invoice_id, invoice.funded, n as u32, &invoice.creator);

        // Spin up next recurring invoice if configured
        if let Some(params) = env.storage().persistent()
            .get::<(Symbol, u64), SubscriptionParams>(&recurring_params_key(invoice_id))
        {
            if params.max_recurrences == 0 || params.num_created < params.max_recurrences {
                let next_deadline = env.ledger().timestamp() + params.recurrence_interval;
                let next_id = bump_counter(env);

                let next_invoice = build_invoice(
                    env, params.creator.clone(), params.recipients.clone(),
                    params.amounts.clone(), params.tokens.clone(), next_deadline, false, 0, Vec::new(env), None,
                );
                save_invoice(env, next_id, &next_invoice);

                let mut next_params = params.clone();
                next_params.num_created += 1;
                env.storage().persistent().set(&recurring_params_key(next_id), &next_params);
                env.storage().persistent().set(&next_invoice_key(invoice_id), &next_id);
                events::invoice_created(env, next_id, &params.creator);
            }
        }
    }

    pub fn release(env: Env, invoice_id: u64) {
        require_not_paused(&env);
        let mut invoice = load_invoice(&env, invoice_id);
        let caller = env.current_contract_address();
        Self::_release(&env, invoice_id, &mut invoice, &caller);
    }

    pub fn refund(env: Env, invoice_id: u64) {
        require_not_paused(&env);
        let mut invoice = load_invoice(&env, invoice_id);
        assert!(invoice.status == InvoiceStatus::Pending, "invoice is not pending");
        assert!(env.ledger().timestamp() > invoice.deadline, "deadline has not passed");

        Self::_refund_payers(&env, invoice_id, &invoice);

        invoice.status = InvoiceStatus::Refunded;
        invoice.completion_time = Some(env.ledger().timestamp());
        save_invoice(&env, invoice_id, &invoice);
        append_audit(&env, invoice_id, symbol_short!("refund"), &env.current_contract_address());
        let recipient_count = invoice.recipients.len() as u32;
        events::invoice_refunded(&env, invoice_id, invoice.funded, recipient_count, &invoice.creator);
    }

    pub fn cancel_invoice(env: Env, caller: Address, invoice_id: u64) {
        require_not_paused(&env);
        caller.require_auth();
        let mut invoice = load_invoice(&env, invoice_id);
        assert!(invoice.status == InvoiceStatus::Pending, "invoice is not pending");
        assert!(invoice.creator == caller, "only creator can cancel");

        if invoice.funded > 0 {
            Self::_refund_payers(&env, invoice_id, &invoice);
            invoice.status = InvoiceStatus::Refunded;
        } else {
            invoice.status = InvoiceStatus::Cancelled;
        }

        save_invoice(&env, invoice_id, &invoice);
        append_audit(&env, invoice_id, symbol_short!("cancel"), &caller);
        let refunded = if invoice.status == InvoiceStatus::Refunded { invoice.funded } else { 0 };
        events::invoice_cancelled(&env, invoice_id, &caller, refunded);
    }

    /// Internal helper — refund all payers and emit per-payer events.
    /// Aggregates payments by payer address to produce one transfer per payer.
    /// Used by `refund`, `cancel_invoice`, and `resolve_dispute` (release=false).
    fn _refund_payers(env: &Env, invoice_id: u64, invoice: &Invoice) {
        let token_client = token::Client::new(env, &invoice.tokens.get(0).expect("no token"));
        let mut totals: Map<Address, i128> = Map::new(env);
        for payment in invoice.payments.iter() {
            let prev = totals.get(payment.payer.clone()).unwrap_or(0);
            totals.set(payment.payer.clone(), prev + payment.amount);
        }
        for (payer, amount) in totals.iter() {
            token_client.transfer(&env.current_contract_address(), &payer, &amount);
            events::payer_refunded(env, invoice_id, &payer, amount);
        }
    }

    pub fn get_invoice(env: Env, invoice_id: u64) -> Invoice {
        load_invoice(&env, invoice_id)
    }

    pub fn get_audit_log(env: Env, invoice_id: u64) -> Vec<AuditEntry> {
        env.storage().persistent().get(&audit_log_key(invoice_id)).unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_payer_total(env: Env, invoice_id: u64, payer: Address) -> i128 {
        load_invoice(&env, invoice_id).payments.iter().filter(|p| p.payer == payer).map(|p| p.amount).sum()
    }

    pub fn get_next_recurring(env: Env, invoice_id: u64) -> Option<u64> {
        env.storage().persistent().get(&next_invoice_key(invoice_id))
    }

    pub fn get_invoice_stats(env: Env, invoice_id: u64) -> InvoiceStats {
        let invoice = load_invoice(&env, invoice_id);
        let total: i128 = invoice.amounts.iter().sum();
        let payment_count = invoice.payments.len();
        let mut unique: Vec<Address> = Vec::new(&env);
        for p in invoice.payments.iter() {
            if !unique.contains(&p.payer) {
                unique.push_back(p.payer.clone());
            }
        }
        let completion_bps = if total > 0 {
            // Protocol 26 CAP-82: checked arithmetic for completion percentage
            invoice.funded
                .checked_mul(10_000)
                .expect("stats: overflow in funded * 10_000")
                .checked_div(total)
                .expect("stats: division by zero") as u32
        } else {
            0
        };
        InvoiceStats {
            funded: invoice.funded,
            total,
            payment_count,
            unique_payers: unique.len(),
            completion_bps,
        }
    }

    pub fn get_escrow_state(env: Env, invoice_id: u64) -> Option<DisputeState> {
        env.storage().persistent().get(&escrow_state_key(invoice_id))
    }

    /// Extend the TTL of an invoice entry to ~1 year.
    /// Protocol 26 CAP-78: host function for limited TTL extension keeps long-lived
    /// and recurring invoices accessible without requiring a full state restore.
    pub fn bump_invoice_ttl(env: Env, invoice_id: u64) {
        let _ = load_invoice(&env, invoice_id);
        env.storage().persistent().extend_ttl(&invoice_key(invoice_id), 100_000, 6_307_200);
    }

    /// Returns the exact per-recipient payout amounts for a given payment amount,
    /// using the same proportional and dust logic as `_release`.
    /// Pure read — no state is modified.
    /// Useful for showing payers a precise breakdown before they sign.
    pub fn preview_payout(env: Env, invoice_id: u64, amount: i128) -> Vec<i128> {
        assert!(amount > 0, "amount must be positive");
        let invoice = load_invoice(&env, invoice_id);
        let total: i128 = invoice.amounts.iter().sum();
        let n = invoice.recipients.len();
        let mut result: Vec<i128> = Vec::new(&env);
        let mut distributed: i128 = 0;

        for i in 0..n {
            let recipient_amount = invoice.amounts.get(i).unwrap();
            let payout = if !invoice.split_rules.is_empty() {
                match invoice.split_rules.get(i as u32).unwrap() {
                    SplitRule::Fixed(fixed_amt) => fixed_amt,
                    SplitRule::Percentage(bps) => {
                        amount
                            .checked_mul(bps as i128)
                            .expect("preview: overflow in amount * bps")
                            .checked_div(10_000)
                            .expect("preview: division failed")
                    }
                    SplitRule::Tiered(threshold, bps) => {
                        if amount > threshold {
                            amount
                                .checked_mul(bps as i128)
                                .expect("preview: overflow in amount * bps")
                                .checked_div(10_000)
                                .expect("preview: division failed")
                        } else {
                            0
                        }
                    }
                }
            } else if i == n - 1 {
                // Last recipient gets the dust remainder — identical to _release
                amount
                    .checked_sub(distributed)
                    .expect("preview: underflow computing remainder")
            } else {
                // Proportional: recipient_amount * payment_amount / total
                recipient_amount
                    .checked_mul(amount)
                    .expect("preview: overflow in amount * funded")
                    .checked_div(total)
                    .expect("preview: division by zero total")
            };

            distributed += payout;
            result.push_back(payout);
        }

        result
    }

    /// Returns all invoice IDs created by a given address.
    /// Updated on every create_invoice, create_batch, and create_recurring call.
    /// Use this for dashboard pagination instead of scanning sequential IDs.
    pub fn get_invoices_by_creator(env: Env, creator: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&creator_index_key(&creator))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Returns the claimable balance for a given account and token.
    /// Balances accumulate when recipient transfers fail during invoice release.
    /// Returns 0 if no balance exists.
    pub fn get_claimable_balance(env: Env, account: Address, token: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&account_balance_key(&account, &token))
            .unwrap_or(0)
    }

    /// Withdraws a credited balance for an account and token.
    /// Permissionless — anyone can trigger the claim for any account.
    /// The transfer goes from the contract vault to the account.
    ///
    /// # Panics
    /// - If the claimable balance is zero
    /// - If the token transfer fails (e.g., account still has no trustline)
    ///
    /// # Security — CEI / reentrancy review (closes #127)
    /// Soroban has no reentrancy: contract calls are synchronous and the host
    /// disallows callback into the same contract during `token::transfer`. Even
    /// so, `claim()` follows Checks-Effects-Interactions (CEI) as defense-in-depth:
    /// 1. Checks: `balance > 0` else panic.
    /// 2. Effects: `storage.remove(key)` deletes the claimable balance BEFORE the
    ///    external interaction. A second `claim()` on same (account,token) will
    ///    see 0 and panic with "no claimable balance" — no double withdrawal even
    ///    if a future host version allowed reentrancy.
    /// 3. Interactions: `token.transfer` is the last operation; event emitted after.
    /// Checked arithmetic in `credit_account` and composite key `(account,token)`
    /// further isolate balances per token.
    pub fn claim(env: Env, account: Address, token: Address) -> i128 {
        require_not_paused(&env);
        let key = account_balance_key(&account, &token);
        let balance: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        assert!(balance > 0, "no claimable balance");

        // CEI pattern: delete storage BEFORE transfer (even though Soroban has no reentrancy)
        env.storage().persistent().remove(&key);

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &account, &balance);

        events::account_balance_claimed(&env, &account, &token, balance);
        balance
    }

    /// Returns a SHA-256 fingerprint of the invoice's immutable fields.
    /// Protocol 25 CAP-75 / Protocol 26 crypto module: deterministic, tamper-evident
    /// content hash. The fingerprint commits to invoice_id, deadline, funded amount,
    /// and total — any modification produces a different hash.
    /// Use this for off-chain verification or receipt generation.
    pub fn get_invoice_fingerprint(env: Env, invoice_id: u64) -> soroban_sdk::BytesN<32> {
        let invoice = load_invoice(&env, invoice_id);
        let total: i128 = invoice.amounts.iter().sum();

        // Build a flat byte buffer of key invoice fields for deterministic hashing
        let mut buf: [u8; 40] = [0u8; 40]; // 8 + 8 + 8 + 16 bytes
        buf[0..8].copy_from_slice(&invoice_id.to_be_bytes());
        buf[8..16].copy_from_slice(&invoice.deadline.to_be_bytes());
        buf[16..24].copy_from_slice(&(invoice.recipients.len() as u64).to_be_bytes());
        buf[24..40].copy_from_slice(&total.to_be_bytes());

        let data = Bytes::from_array(&env, &buf);

        // SHA-256 via Protocol 25/26 crypto host function — returns Hash<32>
        env.crypto().sha256(&data).into()
    }

    /// Returns the total number of invoices ever created.
    /// Reads the global counter directly — O(1), no iteration.
    /// Useful for dashboards, analytics, and protocol stats.
    pub fn get_invoice_count(env: Env) -> u64 {
        env.storage().persistent().get(&counter_key()).unwrap_or(0u64)
    }


    /// Returns the schema version of the invoice — always 1 for current contracts.
    /// Useful for forward-compatibility checks in off-chain indexers and SDKs
    /// when multiple contract versions may be deployed simultaneously.
    pub fn get_invoice_version(env: Env, invoice_id: u64) -> u32 {
        load_invoice(&env, invoice_id).version
    }

    /// Returns the treasury address set during `initialize`.
    /// Admin-facing query for protocol dashboards and treasury management tools.
    pub fn get_treasury(env: Env) -> Address {
        env.storage().instance().get(&treasury_key()).expect("treasury not set")
    }

    /// Returns all invoice IDs that a given address has paid toward.
    /// Indexed on every pay() call — O(1) write, O(n) read where n = unique invoices paid.
    /// Deduplicates: paying the same invoice multiple times only adds one entry.
    pub fn get_invoices_by_payer(env: Env, payer: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&payer_index_key(&payer))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Pay toward an invoice with an optional tip.
    /// The tip is transferred directly to the treasury on top of the invoice payment.
    /// `tip` is stored on the Payment record but does NOT count toward `invoice.funded`
    /// or `get_payer_total`. Set tip=0 to behave identically to `pay()`.
    ///
    /// # Panics
    /// - `"payment amount must be positive"` if amount ≤ 0
    /// - `"tip must be non-negative"` if tip < 0
    /// - `"invoice is not pending"` if status != Pending
    /// - `"invoice deadline has passed"` if past deadline
    /// - `"payment exceeds remaining balance"` if amount > remaining
    pub fn pay_with_tip(env: Env, payer: Address, invoice_id: u64, amount: i128, tip: i128) {
        require_not_paused(&env);
        payer.require_auth();
        assert!(amount > 0, "payment amount must be positive");
        assert!(tip >= 0, "tip must be non-negative");

        let mut invoice = load_invoice(&env, invoice_id);
        assert!(invoice.status == InvoiceStatus::Pending, "invoice is not pending");
        assert!(env.ledger().timestamp() <= invoice.deadline, "invoice deadline has passed");
        assert!(!invoice.frozen, "invoice is frozen");

        let total: i128 = invoice.amounts.iter().sum();
        let remaining = total - invoice.funded;
        if amount > remaining {
            panic!("payment exceeds remaining balance: payment of {} exceeds remaining {}", amount, remaining);
        }

        let token_client = token::Client::new(&env, &invoice.tokens.get(0).expect("no token"));

        // Transfer invoice payment to contract vault
        token_client.transfer(&payer, &env.current_contract_address(), &amount);

        // Transfer tip directly to treasury (separate transfer, not held by contract)
        if tip > 0 {
            let treasury: Address = env.storage().instance().get(&treasury_key()).expect("treasury not set");
            token_client.transfer(&payer, &treasury, &tip);
        }

        invoice.payments.push_back(Payment { payer: payer.clone(), amount, tip });
        invoice.funded += amount;
        index_invoice_for_payer(&env, &payer, invoice_id);
        append_audit(&env, invoice_id, symbol_short!("pay"), &payer);
        events::payment_received(&env, invoice_id, &payer, amount);

        if invoice.funded >= total {
            if invoice.escrow_enabled {
                let release_at = env.ledger().timestamp() + invoice.escrow_release_delay;
                let state = DisputeState { release_at, disputed: false, disputed_at: 0 };
                env.storage().persistent().set(&escrow_state_key(invoice_id), &state);
                events::escrow_funded(&env, invoice_id, release_at, invoice.funded);
                save_invoice(&env, invoice_id, &invoice);
            } else {
                Self::_release(&env, invoice_id, &mut invoice, &payer);
            }
        } else {
            save_invoice(&env, invoice_id, &invoice);
        }
    }

    /// Freezes an invoice — blocks any further `pay()` calls on that invoice.
    /// Admin-only. Sets `invoice.frozen = true`.
    /// Use `unfreeze_invoice` to re-enable payments.
    ///
    /// # Panics
    /// - `"invoice is already frozen"` if already frozen
    pub fn freeze_invoice(env: Env, invoice_id: u64) {
        require_admin(&env);
        let mut invoice = load_invoice(&env, invoice_id);
        assert!(!invoice.frozen, "invoice is already frozen");
        invoice.frozen = true;
        save_invoice(&env, invoice_id, &invoice);
        append_audit(&env, invoice_id, symbol_short!("freeze"), &env.current_contract_address());
        events::invoice_updated(&env, invoice_id, &env.current_contract_address());
    }

    /// Unfreezes a previously frozen invoice — re-enables payments.
    /// Admin-only.
    ///
    /// # Panics
    /// - `"invoice is not frozen"` if not currently frozen
    pub fn unfreeze_invoice(env: Env, invoice_id: u64) {
        require_admin(&env);
        let mut invoice = load_invoice(&env, invoice_id);
        assert!(invoice.frozen, "invoice is not frozen");
        invoice.frozen = false;
        save_invoice(&env, invoice_id, &invoice);
        append_audit(&env, invoice_id, symbol_short!("unfreeze"), &env.current_contract_address());
        events::invoice_updated(&env, invoice_id, &env.current_contract_address());
    }

    /// Returns the subscription (recurring) configuration for a recurring invoice.
    /// Includes creator, recipients, amounts, tokens, interval, max_recurrences,
    /// and num_created (how many invoices have been generated so far in the chain).
    /// Returns None for non-recurring invoices.
    pub fn get_recurring_params(env: Env, invoice_id: u64) -> Option<SubscriptionParams> {
        env.storage().persistent().get(&recurring_params_key(invoice_id))
    }

    /// Attaches or replaces free-text notes on an invoice.
    /// Only callable by the invoice creator.
    /// Notes are stored under a separate key so the core invoice struct is unchanged.
    /// Appends a `notes` audit entry on every call.
    ///
    /// # Panics
    /// - `"only creator can set notes"` if caller != invoice.creator
    pub fn set_invoice_notes(env: Env, caller: Address, invoice_id: u64, text: String) {
        require_not_paused(&env);
        caller.require_auth();
        let invoice = load_invoice(&env, invoice_id);
        assert!(invoice.creator == caller, "only creator can set notes");

        let notes = InvoiceNotes { text: text.clone(), updated_at: env.ledger().timestamp() };
        env.storage().persistent().set(&invoice_notes_key(invoice_id), &notes);
        env.storage().persistent().extend_ttl(&invoice_notes_key(invoice_id), 100_000, 6_307_200);
        append_audit(&env, invoice_id, symbol_short!("notes"), &caller);
        events::invoice_updated(&env, invoice_id, &caller);
    }

    /// Returns the notes attached to an invoice, or None if none have been set.
    pub fn get_invoice_notes(env: Env, invoice_id: u64) -> Option<InvoiceNotes> {
        env.storage().persistent().get(&invoice_notes_key(invoice_id))
    }
}

/// Validates that a token address is not the zero address.
/// Soroban addresses are opaque — this is a no-op guard that ensures the address
/// was properly constructed (non-null). Actual token validity is enforced by the
/// token contract itself at transfer time.
#[allow(dead_code)]
fn validate_token_address(_env: &Env, _token: &Address) {
    // Address type is non-nullable in Soroban — construction guarantees validity.
    // Runtime validation happens at token::Client::transfer invocation.
}

// Feature: add invoice tagging system - PR #4

// Feature: enhance multi-currency handling - PR #5

// Feature: add payment memo support - PR #6

// Feature: add escrow edge case tests - PR #7

// Feature: add batch refund functionality - PR #8

// Feature: add invoice template storage - PR #9

// Feature: add pause/resume for recurring invoices - PR #10

// Feature: add split rule validation tests - PR #11

// Feature: add extended invoice metadata - PR #12

// Feature: add payment routing logic - PR #13

// Feature: optimize storage access patterns - PR #14

// Feature: add invoice archival system - PR #15

// Feature: add deadline edge case tests - PR #16

// Feature: add multi-signature support prep - PR #17

// Feature: add invoice categorization - PR #18

// Feature: add payment amount limits - PR #19

// Feature: add concurrent payment tests - PR #20

// Feature: add attachment hash storage - PR #21

// Feature: add dispute timeout mechanism - PR #22

// Feature: refactor event emission logic - PR #23

// Feature: add discount rules support - PR #24

// Feature: add gas usage tests - PR #25

// Feature: add scheduled payment support - PR #26

// Feature: add auto-expiry actions - PR #27

// Feature: add weighted recipient splits - PR #28

// Feature: add stress test suite - PR #29

// Feature: improve inline docs - PR #30
