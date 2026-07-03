//! Sharpy — Advanced split payment contract with recurring splits, escrow, and batch operations.

#![no_std]

mod events;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, Env, Map, Symbol, Vec};
use types::{
    AuditEntry, CreateInvoiceParams, DisputeState, Invoice, InvoiceOptions, InvoicePayment,
    InvoiceStats, InvoiceStatus, Payment, SplitRule, SubscriptionParams,
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

fn is_paused(env: &Env) -> bool {
    env.storage().persistent().get(&paused_key()).unwrap_or(false)
}

fn require_not_paused(env: &Env) {
    assert!(!is_paused(env), "contract is paused");
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
    // Extend TTL to ~1 year (in ledgers at ~5s each: 365*24*3600/5 = 6_307_200)
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

    pub fn pay(env: Env, payer: Address, invoice_id: u64, amount: i128) {
        require_not_paused(&env);
        payer.require_auth();
        assert!(amount > 0, "payment amount must be positive");

        let mut invoice = load_invoice(&env, invoice_id);
        assert!(invoice.status == InvoiceStatus::Pending, "invoice is not pending");
        assert!(env.ledger().timestamp() <= invoice.deadline, "invoice deadline has passed");

        let total: i128 = invoice.amounts.iter().sum();
        assert!(amount <= total - invoice.funded, "payment exceeds remaining balance");

        let token_client = token::Client::new(&env, &invoice.tokens.get(0).expect("no token"));
        token_client.transfer(&payer, &env.current_contract_address(), &amount);

        invoice.payments.push_back(Payment { payer: payer.clone(), amount, tip: 0 });
        invoice.funded += amount;
        append_audit(&env, invoice_id, symbol_short!("pay"), &payer);
        events::payment_received(&env, invoice_id, &payer, amount);

        if invoice.funded >= total {
            if invoice.escrow_enabled {
                let release_at = env.ledger().timestamp() + invoice.escrow_release_delay;
                let state = DisputeState { release_at, disputed: false, disputed_at: 0 };
                env.storage().persistent().set(&escrow_state_key(invoice_id), &state);
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
            assert!(inv.funded + p.amount <= inv_total, "payment exceeds remaining balance");
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
            append_audit(&env, p.invoice_id, symbol_short!("pool_pay"), &payer);
            events::payment_received(&env, p.invoice_id, &payer, p.amount);
            let inv_total: i128 = inv.amounts.iter().sum();
            if inv.funded >= inv_total {
                if inv.escrow_enabled {
                    let release_at = env.ledger().timestamp() + inv.escrow_release_delay;
                    let state = DisputeState { release_at, disputed: false, disputed_at: 0 };
                    env.storage().persistent().set(&escrow_state_key(p.invoice_id), &state);
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
            let token_client = token::Client::new(&env, &invoice.tokens.get(0).expect("no token"));
            let mut totals: Map<Address, i128> = Map::new(&env);
            for payment in invoice.payments.iter() {
                let prev = totals.get(payment.payer.clone()).unwrap_or(0);
                totals.set(payment.payer.clone(), prev + payment.amount);
            }
            for (payer, amount) in totals.iter() {
                token_client.transfer(&env.current_contract_address(), &payer, &amount);
                events::payer_refunded(&env, invoice_id, &payer, amount);
            }

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
            let token_client = token::Client::new(env, &invoice.tokens.get(i).expect("no token"));

            let proportional = if !invoice.split_rules.is_empty() {
                match invoice.split_rules.get(i as u32).unwrap() {
                    SplitRule::Fixed(fixed_amt) => fixed_amt,
                    SplitRule::Percentage(bps) => {
                        (invoice.funded as u128 * bps as u128 / 10_000u128) as i128
                    }
                    SplitRule::Tiered(threshold, bps) => {
                        if invoice.funded > threshold {
                            (invoice.funded as u128 * bps as u128 / 10_000u128) as i128
                        } else {
                            0
                        }
                    }
                }
            } else if i == n - 1 {
                invoice.funded - distributed
            } else {
                (amount as u128 * invoice.funded as u128 / total as u128) as i128
            };

            distributed += proportional;
            if proportional > 0 {
                token_client.transfer(&env.current_contract_address(), &recipient, &proportional);
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

        let token_client = token::Client::new(&env, &invoice.tokens.get(0).expect("no token"));
        let mut totals: Map<Address, i128> = Map::new(&env);
        for payment in invoice.payments.iter() {
            let prev = totals.get(payment.payer.clone()).unwrap_or(0);
            totals.set(payment.payer.clone(), prev + payment.amount);
        }
        for (payer, amount) in totals.iter() {
            token_client.transfer(&env.current_contract_address(), &payer, &amount);
            events::payer_refunded(&env, invoice_id, &payer, amount);
        }

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
            let token_client = token::Client::new(&env, &invoice.tokens.get(0).expect("no token"));
            let mut totals: Map<Address, i128> = Map::new(&env);
            for payment in invoice.payments.iter() {
                let prev = totals.get(payment.payer.clone()).unwrap_or(0);
                totals.set(payment.payer.clone(), prev + payment.amount);
            }
            for (payer, amount) in totals.iter() {
                token_client.transfer(&env.current_contract_address(), &payer, &amount);
                events::payer_refunded(&env, invoice_id, &payer, amount);
            }
            invoice.status = InvoiceStatus::Refunded;
        } else {
            invoice.status = InvoiceStatus::Cancelled;
        }

        save_invoice(&env, invoice_id, &invoice);
        append_audit(&env, invoice_id, symbol_short!("cancel"), &caller);
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
            (invoice.funded as u128 * 10_000u128 / total as u128) as u32
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
}
