//! Sharpy — Advanced split payment contract with recurring splits, escrow, and batch operations.

#![no_std]

mod events;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{
    String,
    contract, contractimpl, symbol_short, token, Address, Bytes, BytesN, Env, IntoVal, Map, Symbol, Val, Vec,
};
use types::{
    AuditEntry, CreateInvoiceParams, Invoice, InvoiceOptions, InvoicePayment, InvoiceStats,
    InvoiceStatus, Payment, ResolveAction, ResolveRule, SplitRule, SubscriptionParams, Tranche,
};

// ---------------------------------------------------------------------------
// Storage key helpers
// ---------------------------------------------------------------------------

fn admin_key() -> Symbol {
    symbol_short!("admin")
}

fn paused_key() -> Symbol {
    symbol_short!("paused")
}

fn treasury_key() -> Symbol {
    symbol_short!("treasury")
}

fn counter_key() -> Symbol {
    symbol_short!("counter")
}

fn invoice_key(id: u64) -> (Symbol, u64) {
    (symbol_short!("inv"), id)
}

fn audit_log_key(id: u64) -> (Symbol, u64) {
    (symbol_short!("log"), id)
}

fn escrow_state_key(invoice_id: u64) -> (Symbol, u64) {
    (symbol_short!("escrow"), invoice_id)
}

fn recurring_params_key(id: u64) -> (Symbol, u64) {
    (symbol_short!("rec"), id)
}

fn next_invoice_key(id: u64) -> (Symbol, u64) {
    (symbol_short!("next_inv"), id)
}

// ---------------------------------------------------------------------------
// Admin / pause helpers
// ---------------------------------------------------------------------------

fn is_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&paused_key())
        .unwrap_or(false)
}

fn require_not_paused(env: &Env) {
    assert!(!is_paused(env), "contract is paused");
}

fn require_admin(env: &Env) -> Address {
    let admin: Address = env
        .storage()
        .instance()
        .get(&admin_key())
        .expect("admin not set");
    admin.require_auth();
    admin
}

// ---------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------

fn load_invoice(env: &Env, id: u64) -> Invoice {
    env.storage()
        .persistent()
        .get(&invoice_key(id))
        .expect("invoice not found")
}

fn save_invoice(env: &Env, id: u64, invoice: &Invoice) {
    env.storage().persistent().set(&invoice_key(id), invoice);
}

fn append_audit_entry(env: &Env, id: u64, action: Symbol, actor: &Address) {
    let timestamp = env.ledger().timestamp();
    let entry = AuditEntry { action, actor: actor.clone(), timestamp };
    let mut log: Vec<AuditEntry> = env
        .storage()
        .persistent()
        .get(&audit_log_key(id))
        .unwrap_or_else(|| Vec::new(env));
    log.push_back(entry);
    env.storage().persistent().set(&audit_log_key(id), &log);
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct SharpyContract;

#[contractimpl]
impl SharpyContract {
    /// Initialize the contract with admin and treasury addresses.
    pub fn initialize(env: Env, admin: Address, treasury: Address) {
        assert!(
            !env.storage().instance().has(&admin_key()),
            "already initialized"
        );
        env.storage().instance().set(&admin_key(), &admin);
        env.storage().instance().set(&treasury_key(), &treasury);
        env.storage().persistent().set(&paused_key(), &false);
    }

    /// Pause the contract. Requires admin auth.
    pub fn pause(env: Env, admin: Address) {
        require_admin(&env);
        let _ = admin;
        env.storage().persistent().set(&paused_key(), &true);
    }

    /// Unpause the contract. Requires admin auth.
    pub fn unpause(env: Env, admin: Address) {
        require_admin(&env);
        let _ = admin;
        env.storage().persistent().set(&paused_key(), &false);
    }

    // -----------------------------------------------------------------------
    // Core Invoice Creation
    // -----------------------------------------------------------------------

    /// Create a single invoice with split rules.
    pub fn create_invoice(
        env: Env,
        creator: Address,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
        token: Address,
        deadline: u64,
        options: InvoiceOptions,
    ) -> u64 {
        require_not_paused(&env);
        creator.require_auth();

        assert_eq!(
            recipients.len(),
            amounts.len(),
            "recipients and amounts length mismatch"
        );
        assert!(!recipients.is_empty(), "must have at least one recipient");
        assert!(
            deadline > env.ledger().timestamp(),
            "deadline must be in the future"
        );

        for amt in amounts.iter() {
            assert!(amt > &0, "amounts must be positive");
        }

        let id: u64 = env
            .storage()
            .persistent()
            .get(&counter_key())
            .unwrap_or(0u64)
            + 1;
        env.storage().persistent().set(&counter_key(), &id);

        // Build token vec (all same token in MVP)
        let mut tokens: Vec<Address> = Vec::new(&env);
        for _ in recipients.iter() {
            tokens.push_back(token.clone());
        }

        // Initialize claimed vec to 0
        let mut claimed: Vec<i128> = Vec::new(&env);
        for _ in recipients.iter() {
            claimed.push_back(0i128);
        }

        let invoice = Invoice {
            version: 1u32,
            creator: creator.clone(),
            recipients: recipients.clone(),
            amounts: amounts.clone(),
            tokens,
            deadline,
            funded: 0,
            status: InvoiceStatus::Pending,
            payments: Vec::new(&env),
            claimed,
            frozen: false,
            completion_time: None,
            escrow_enabled: options.escrow_enabled,
            escrow_release_delay: options.escrow_release_delay.unwrap_or(0),
            split_rules: options.split_rules.clone(),
            auto_resolve_rules: options.auto_resolve_rules.clone(),
        };

        save_invoice(&env, id, &invoice);
        events::invoice_created(&env, id, &creator);

        id
    }

    /// Create multiple invoices in a single transaction (up to 10).
    pub fn create_batch(
        env: Env,
        creator: Address,
        invoices: Vec<CreateInvoiceParams>,
    ) -> Vec<u64> {
        require_not_paused(&env);
        creator.require_auth();
        assert!(invoices.len() <= 10, "batch limit is 10");

        let mut ids: Vec<u64> = Vec::new(&env);
        for params in invoices.iter() {
            let id: u64 = env
                .storage()
                .persistent()
                .get(&counter_key())
                .unwrap_or(0u64)
                + 1;
            env.storage().persistent().set(&counter_key(), &id);

            let mut tokens: Vec<Address> = Vec::new(&env);
            for _ in params.recipients.iter() {
                tokens.push_back(params.token.clone());
            }

            let mut claimed: Vec<i128> = Vec::new(&env);
            for _ in params.recipients.iter() {
                claimed.push_back(0i128);
            }

            let invoice = Invoice {
                version: 1u32,
                creator: creator.clone(),
                recipients: params.recipients.clone(),
                amounts: params.amounts.clone(),
                tokens,
                deadline: params.deadline,
                funded: 0,
                status: InvoiceStatus::Pending,
                payments: Vec::new(&env),
                claimed,
                frozen: false,
                completion_time: None,
                escrow_enabled: false,
                escrow_release_delay: 0,
                split_rules: Vec::new(&env),
                auto_resolve_rules: Vec::new(&env),
            };

            save_invoice(&env, id, &invoice);
            events::invoice_created(&env, id, &creator);
            ids.push_back(id);
        }
        ids
    }

    /// Create a recurring invoice that auto-generates the next invoice upon release.
    pub fn create_recurring(
        env: Env,
        creator: Address,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
        token: Address,
        deadline: u64,
        recurrence_interval: u64, // seconds between invoices
        max_recurrences: u32,    // 0 = infinite
    ) -> u64 {
        require_not_paused(&env);
        creator.require_auth();

        assert_eq!(
            recipients.len(),
            amounts.len(),
            "recipients and amounts length mismatch"
        );
        assert!(recurrence_interval > 0, "recurrence_interval must be positive");

        let id: u64 = env
            .storage()
            .persistent()
            .get(&counter_key())
            .unwrap_or(0u64)
            + 1;
        env.storage().persistent().set(&counter_key(), &id);

        let mut tokens: Vec<Address> = Vec::new(&env);
        for _ in recipients.iter() {
            tokens.push_back(token.clone());
        }

        let mut claimed: Vec<i128> = Vec::new(&env);
        for _ in recipients.iter() {
            claimed.push_back(0i128);
        }

        let invoice = Invoice {
            version: 1u32,
            creator: creator.clone(),
            recipients: recipients.clone(),
            amounts: amounts.clone(),
            tokens,
            deadline,
            funded: 0,
            status: InvoiceStatus::Pending,
            payments: Vec::new(&env),
            claimed,
            frozen: false,
            completion_time: None,
            escrow_enabled: false,
            escrow_release_delay: 0,
            split_rules: Vec::new(&env),
            auto_resolve_rules: Vec::new(&env),
        };

        save_invoice(&env, id, &invoice);

        // Store recurring params
        let params = SubscriptionParams {
            creator,
            recipients,
            amounts,
            tokens: Vec::from_array(&env, &[token]),
            recurrence_interval,
            max_recurrences,
            num_created: 1,
        };
        env.storage()
            .persistent()
            .set(&recurring_params_key(id), &params);

        events::invoice_created(&env, id, &env.current_contract_address());
        id
    }

    // -----------------------------------------------------------------------
    // Payment
    // -----------------------------------------------------------------------

    /// Pay toward an invoice.
    pub fn pay(env: Env, payer: Address, invoice_id: u64, amount: i128) {
        require_not_paused(&env);
        payer.require_auth();
        assert!(amount > 0, "payment amount must be positive");

        let mut invoice = load_invoice(&env, invoice_id);
        assert!(
            invoice.status == InvoiceStatus::Pending,
            "invoice is not pending"
        );
        assert!(
            env.ledger().timestamp() <= invoice.deadline,
            "invoice deadline has passed"
        );

        let total: i128 = invoice.amounts.iter().sum();
        let remaining = total - invoice.funded;
        assert!(amount <= remaining, "payment exceeds remaining balance");

        let token_client = token::Client::new(&env, &invoice.tokens.get(0).expect("no token"));
        token_client.transfer(&payer, &env.current_contract_address(), &amount);

        invoice.payments.push_back(Payment { payer: payer.clone(), amount, tip: 0 });
        invoice.funded += amount;

        append_audit_entry(&env, invoice_id, symbol_short!("pay"), &payer);
        events::payment_received(&env, invoice_id, &payer, amount);

        if invoice.funded >= total {
            // Check if escrow is enabled
            if invoice.escrow_enabled {
                // Store escrow state
                let escrow_release_at = env.ledger().timestamp() + invoice.escrow_release_delay;
                env.storage()
                    .persistent()
                    .set(&escrow_state_key(invoice_id), &escrow_release_at);
                save_invoice(&env, invoice_id, &invoice);
            } else {
                // Auto-release if no escrow
                Self::_release(&env, invoice_id, &mut invoice, &payer);
            }
        } else {
            save_invoice(&env, invoice_id, &invoice);
        }
    }

    /// Pay toward multiple invoices in a single call.
    pub fn pool_pay(env: Env, payer: Address, payments: Vec<InvoicePayment>) {
        require_not_paused(&env);
        payer.require_auth();
        assert!(!payments.is_empty(), "payments must not be empty");

        let mut total: i128 = 0;
        for p in payments.iter() {
            let inv = load_invoice(&env, p.invoice_id);
            assert!(inv.status == InvoiceStatus::Pending, "invoice is not pending");
            assert!(p.amount > 0, "payment amount must be positive");
            let inv_total: i128 = inv.amounts.iter().sum();
            assert!(
                inv.funded + p.amount <= inv_total,
                "payment exceeds remaining balance"
            );
            total += p.amount;
        }

        // Single token transfer
        let first_inv = load_invoice(&env, payments.get(0).unwrap().invoice_id);
        let token_client = token::Client::new(&env, &first_inv.tokens.get(0).expect("no token"));
        token_client.transfer(&payer, &env.current_contract_address(), &total);

        // Update each invoice
        for p in payments.iter() {
            let mut inv = load_invoice(&env, p.invoice_id);
            inv.payments.push_back(Payment { payer: payer.clone(), amount: p.amount, tip: 0 });
            inv.funded += p.amount;

            append_audit_entry(&env, p.invoice_id, symbol_short!("pool_pay"), &payer);
            events::payment_received(&env, p.invoice_id, &payer, p.amount);

            let inv_total: i128 = inv.amounts.iter().sum();
            if inv.funded >= inv_total && !inv.escrow_enabled {
                Self::_release(&env, p.invoice_id, &mut inv, &payer);
            } else {
                save_invoice(&env, p.invoice_id, &inv);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Escrow Release
    // -----------------------------------------------------------------------

    /// Release an escrow-held invoice if the delay has passed.
    pub fn release_escrow(env: Env, invoice_id: u64) {
        require_not_paused(&env);
        let mut invoice = load_invoice(&env, invoice_id);
        assert!(invoice.escrow_enabled, "escrow not enabled on this invoice");

        let escrow_release_at: u64 = env
            .storage()
            .persistent()
            .get(&escrow_state_key(invoice_id))
            .expect("escrow not found");
        assert!(
            env.ledger().timestamp() >= escrow_release_at,
            "escrow delay not yet met"
        );

        Self::_release(&env, invoice_id, &mut invoice, &env.current_contract_address());
        env.storage()
            .persistent()
            .remove(&escrow_state_key(invoice_id));
    }

    // -----------------------------------------------------------------------
    // Release
    // -----------------------------------------------------------------------

    fn _release(env: &Env, invoice_id: u64, invoice: &mut Invoice, actor: &Address) {
        assert!(
            invoice.status == InvoiceStatus::Pending,
            "invoice is not pending"
        );

        let token_client = token::Client::new(env, &invoice.tokens.get(0).expect("no token"));
        let total: i128 = invoice.amounts.iter().sum();
        let n = invoice.recipients.len();
        let mut distributed: i128 = 0;

        for i in 0..n {
            let recipient = invoice.recipients.get(i).unwrap();
            let amount = invoice.amounts.get(i).unwrap();

            let proportional = if !invoice.split_rules.is_empty() {
                let rule = invoice.split_rules.get(i as u32).unwrap();
                match rule {
                    SplitRule::Fixed(fixed_amt) => fixed_amt,
                    SplitRule::Percentage(bps) => {
                        (invoice.funded as u128 * bps as u128 / 10_000u128) as i128
                    }
                    SplitRule::Tiered { threshold, bps } => {
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
                token_client.transfer(env.current_contract_address(), &recipient, &proportional);
            }
        }

        invoice.status = InvoiceStatus::Released;
        invoice.completion_time = Some(env.ledger().timestamp());
        save_invoice(env, invoice_id, invoice);
        append_audit_entry(env, invoice_id, symbol_short!("release"), actor);
        events::invoice_released(env, invoice_id, &invoice.recipients);

        // Spin up next recurring invoice if configured
        if let Some(params) = env
            .storage()
            .persistent()
            .get::<(Symbol, u64), SubscriptionParams>(&recurring_params_key(invoice_id))
        {
            // Only create if under max recurrences
            if params.max_recurrences == 0 || params.num_created < params.max_recurrences {
                let next_deadline = env.ledger().timestamp() + params.recurrence_interval;
                let token = params.tokens.get(0).expect("no token");

                let next_id: u64 = env
                    .storage()
                    .persistent()
                    .get(&counter_key())
                    .unwrap_or(0u64)
                    + 1;
                env.storage().persistent().set(&counter_key(), &next_id);

                let mut tokens: Vec<Address> = Vec::new(env);
                for _ in params.recipients.iter() {
                    tokens.push_back(token.clone());
                }

                let mut claimed: Vec<i128> = Vec::new(env);
                for _ in params.recipients.iter() {
                    claimed.push_back(0i128);
                }

                let next_invoice = Invoice {
                    version: 1u32,
                    creator: params.creator.clone(),
                    recipients: params.recipients.clone(),
                    amounts: params.amounts.clone(),
                    tokens,
                    deadline: next_deadline,
                    funded: 0,
                    status: InvoiceStatus::Pending,
                    payments: Vec::new(env),
                    claimed,
                    frozen: false,
                    completion_time: None,
                    escrow_enabled: false,
                    escrow_release_delay: 0,
                    split_rules: Vec::new(env),
                    auto_resolve_rules: Vec::new(env),
                };

                save_invoice(env, next_id, &next_invoice);

                // Update recurring params
                let mut next_params = params.clone();
                next_params.num_created += 1;
                env.storage()
                    .persistent()
                    .set(&recurring_params_key(next_id), &next_params);

                env.storage()
                    .persistent()
                    .set(&next_invoice_key(invoice_id), &next_id);

                events::invoice_created(env, next_id, &params.creator);
            }
        }
    }

    /// Release funds to recipients.
    pub fn release(env: Env, invoice_id: u64) {
        require_not_paused(&env);
        let mut invoice = load_invoice(&env, invoice_id);
        let caller = env.current_contract_address();
        Self::_release(&env, invoice_id, &mut invoice, &caller);
    }

    // -----------------------------------------------------------------------
    // Refund / Cancel
    // -----------------------------------------------------------------------

    /// Refund all payers if deadline has passed and invoice is not fully funded.
    pub fn refund(env: Env, invoice_id: u64) {
        require_not_paused(&env);
        let mut invoice = load_invoice(&env, invoice_id);

        assert!(
            invoice.status == InvoiceStatus::Pending,
            "invoice is not pending"
        );
        assert!(
            env.ledger().timestamp() > invoice.deadline,
            "deadline has not passed"
        );

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
        append_audit_entry(&env, invoice_id, symbol_short!("refund"), &env.current_contract_address());
        events::invoice_refunded(&env, invoice_id);
    }

    /// Cancel an invoice and refund all payments.
    pub fn cancel_invoice(env: Env, caller: Address, invoice_id: u64) {
        require_not_paused(&env);
        caller.require_auth();

        let mut invoice = load_invoice(&env, invoice_id);
        assert!(
            invoice.status == InvoiceStatus::Pending,
            "invoice is not pending"
        );
        assert!(invoice.creator == caller, "only creator can cancel");

        if invoice.funded > 0 {
            let token_client =
                token::Client::new(&env, &invoice.tokens.get(0).expect("no token"));

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
        append_audit_entry(&env, invoice_id, symbol_short!("cancel"), &caller);
    }

    // -----------------------------------------------------------------------
    // Read-only
    // -----------------------------------------------------------------------

    pub fn get_invoice(env: Env, invoice_id: u64) -> Invoice {
        load_invoice(&env, invoice_id)
    }

    pub fn get_audit_log(env: Env, invoice_id: u64) -> Vec<AuditEntry> {
        env.storage()
            .persistent()
            .get(&audit_log_key(invoice_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_payer_total(env: Env, invoice_id: u64, payer: Address) -> i128 {
        let invoice = load_invoice(&env, invoice_id);
        invoice
            .payments
            .iter()
            .filter(|p| p.payer == payer)
            .map(|p| p.amount)
            .sum()
    }

    pub fn get_next_recurring(env: Env, invoice_id: u64) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&next_invoice_key(invoice_id))
    }
}
