#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, token, Address, Env, Vec};
    use soroban_sdk::testutils::Ledger as _;
    use crate::{
        types::{CreateInvoiceParams, InvoiceOptions, InvoicePayment, InvoiceStatus, SplitRule},
        SharpyContractClient,
    };

    fn setup_with_tokens(
        env: &Env,
        payer: &Address,
        amounts: &[i128],
    ) -> (Address, Address) {
        let admin = Address::generate(env);
        let token_a = env.register_stellar_asset_contract(admin.clone());
        let token_b = env.register_stellar_asset_contract(admin);
        let sac_a = token::StellarAssetClient::new(env, &token_a);
        let sac_b = token::StellarAssetClient::new(env, &token_b);
        sac_a.mint(payer, &amounts[0]);
        sac_b.mint(payer, &amounts[1]);
        (token_a, token_b)
    }

    fn setup() -> (Env, SharpyContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::SharpyContract, ());
        let client = SharpyContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &treasury);
        (env, client)
    }

    fn default_options(env: &Env) -> InvoiceOptions {
        InvoiceOptions {
            escrow_enabled: false,
            escrow_release_delay: None,
            split_rules: Vec::new(env),
            auto_resolve_rules: Vec::new(env),
            arbitrator: None,
        }
    }

    // -----------------------------------------------------------------------
    // Existing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_invoice() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [1000i128]),
            &Vec::from_array(&env, [token]),
            &deadline,
            &default_options(&env),
        );

        assert!(id > 0);
        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Pending);
        assert_eq!(invoice.funded, 0);
    }

    #[test]
    fn test_batch_create() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let params = CreateInvoiceParams {
            recipients: Vec::from_array(&env, [recipient.clone()]),
            amounts: Vec::from_array(&env, [500i128]),
            tokens: Vec::from_array(&env, [token.clone()]),
            deadline,
        };
        let batch = Vec::from_array(&env, [params.clone(), params]);
        let ids = client.create_batch(&creator, &batch);

        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_cancel_invoice() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [1000i128]),
            &Vec::from_array(&env, [token]),
            &deadline,
            &default_options(&env),
        );

        client.cancel_invoice(&creator, &id);
        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Cancelled);
    }

    // -----------------------------------------------------------------------
    // New tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_invoice_ids_increment() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id1 = client.create_invoice(&creator, &Vec::from_array(&env, [recipient.clone()]),
            &Vec::from_array(&env, [100i128]), &Vec::from_array(&env, [token.clone()]),
            &deadline, &default_options(&env));
        let id2 = client.create_invoice(&creator, &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [100i128]), &Vec::from_array(&env, [token]),
            &deadline, &default_options(&env));

        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_create_invoice_stores_creator_and_amounts() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(&creator, &Vec::from_array(&env, [recipient.clone()]),
            &Vec::from_array(&env, [750i128]), &Vec::from_array(&env, [token]),
            &deadline, &default_options(&env));

        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.creator, creator);
        assert_eq!(invoice.amounts.get(0).unwrap(), 750i128);
        assert_eq!(invoice.recipients.get(0).unwrap(), recipient);
    }

    #[test]
    fn test_batch_creates_correct_ids() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let params = CreateInvoiceParams {
            recipients: Vec::from_array(&env, [recipient]),
            amounts: Vec::from_array(&env, [100i128]),
            tokens: Vec::from_array(&env, [token]),
            deadline,
        };
        let batch = Vec::from_array(&env, [params.clone(), params.clone(), params]);
        let ids = client.create_batch(&creator, &batch);

        assert_eq!(ids.len(), 3);
        let id0 = ids.get(0).unwrap();
        let id1 = ids.get(1).unwrap();
        let id2 = ids.get(2).unwrap();
        assert_eq!(id1, id0 + 1);
        assert_eq!(id2, id0 + 2);
    }

    #[test]
    fn test_get_audit_log_records_cancel() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(&creator, &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [500i128]), &Vec::from_array(&env, [token]),
            &deadline, &default_options(&env));

        client.cancel_invoice(&creator, &id);
        let log = client.get_audit_log(&id);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_cancel_funded_invoice_gives_refunded_status() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(&creator, &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [500i128]), &Vec::from_array(&env, [token]),
            &deadline, &default_options(&env));

        client.cancel_invoice(&creator, &id);
        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Cancelled);
    }

    #[test]
    fn test_create_recurring_invoice() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_recurring(
            &creator,
            &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [1000i128]),
            &Vec::from_array(&env, [token]),
            &deadline,
            &(86400u64 * 30),
            &0u32,
        );

        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Pending);
        assert_eq!(invoice.funded, 0);
    }

    #[test]
    fn test_get_next_recurring_none_before_release() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_recurring(
            &creator,
            &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [500i128]),
            &Vec::from_array(&env, [token]),
            &deadline,
            &(86400u64),
            &0u32,
        );

        assert!(client.get_next_recurring(&id).is_none());
    }

    #[test]
    fn test_invoice_deadline_stored_correctly() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 7 * 86400;

        let id = client.create_invoice(&creator, &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [100i128]), &Vec::from_array(&env, [token]),
            &deadline, &default_options(&env));

        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.deadline, deadline);
    }

    #[test]
    fn test_escrow_invoice_creation() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let options = InvoiceOptions {
            escrow_enabled: true,
            escrow_release_delay: Some(3600u64),
            split_rules: Vec::new(&env),
            auto_resolve_rules: Vec::new(&env),
            arbitrator: None,
        };

        let id = client.create_invoice(&creator, &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [1000i128]), &Vec::from_array(&env, [token]),
            &deadline, &options);

        let invoice = client.get_invoice(&id);
        assert!(invoice.escrow_enabled);
        assert_eq!(invoice.escrow_release_delay, 3600u64);
    }

    #[test]
    fn test_multiple_recipients_stored() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let r3 = Address::generate(&env);
        let t1 = Address::generate(&env);
        let t2 = Address::generate(&env);
        let t3 = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [r1.clone(), r2.clone(), r3.clone()]),
            &Vec::from_array(&env, [300i128, 300i128, 400i128]),
            &Vec::from_array(&env, [t1, t2, t3]),
            &deadline,
            &default_options(&env),
        );

        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.recipients.len(), 3);
        assert_eq!(invoice.amounts.get(2).unwrap(), 400i128);
        assert_eq!(invoice.tokens.len(), 3);
    }

    #[test]
    fn test_payer_total_starts_at_zero() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let payer = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(&creator, &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [500i128]), &Vec::from_array(&env, [token]),
            &deadline, &default_options(&env));

        assert_eq!(client.get_payer_total(&id, &payer), 0i128);
    }

    #[test]
    #[should_panic]
    fn test_pool_pay_rejects_overpayment() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let payer = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id1 = client.create_invoice(&creator, &Vec::from_array(&env, [recipient.clone()]),
            &Vec::from_array(&env, [200i128]), &Vec::from_array(&env, [token.clone()]),
            &deadline, &default_options(&env));
        let id2 = client.create_invoice(&creator, &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [300i128]), &Vec::from_array(&env, [token]),
            &deadline, &default_options(&env));

        let payments = Vec::from_array(&env, [
            InvoicePayment { invoice_id: id1, amount: 999i128 },
            InvoicePayment { invoice_id: id2, amount: 100i128 },
        ]);
        client.pool_pay(&payer, &payments);
    }

    #[test]
    fn test_multi_token_invoice_stores_per_recipient_tokens() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let usdc = Address::generate(&env);
        let xlm = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [r1, r2]),
            &Vec::from_array(&env, [500i128, 300i128]),
            &Vec::from_array(&env, [usdc.clone(), xlm.clone()]),
            &deadline,
            &default_options(&env),
        );

        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.tokens.get(0).unwrap(), usdc);
        assert_eq!(invoice.tokens.get(1).unwrap(), xlm);
    }

    #[test]
    #[should_panic]
    fn test_create_invoice_rejects_token_length_mismatch() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        // 2 recipients but only 1 token — should panic
        client.create_invoice(
            &creator,
            &Vec::from_array(&env, [r1, r2]),
            &Vec::from_array(&env, [500i128, 300i128]),
            &Vec::from_array(&env, [token]),
            &deadline,
            &default_options(&env),
        );
    }

    // -----------------------------------------------------------------------
    // Escrow dispute tests
    // -----------------------------------------------------------------------

    fn create_escrow_invoice(
        env: &Env,
        client: &SharpyContractClient<'static>,
        creator: &Address,
        payer: &Address,
        recipient: &Address,
        arbitrator: Option<Address>,
    ) -> (u64, Address) {
        let admin = Address::generate(env);
        let token = env.register_stellar_asset_contract(admin.clone());
        let sac = soroban_sdk::token::StellarAssetClient::new(env, &token);
        sac.mint(payer, &1000i128);

        let deadline = env.ledger().timestamp() + 86400;
        let options = InvoiceOptions {
            escrow_enabled: true,
            escrow_release_delay: Some(3600u64),
            split_rules: Vec::new(env),
            auto_resolve_rules: Vec::new(env),
            arbitrator,
        };

        let id = client.create_invoice(creator, &Vec::from_array(env, [recipient.clone()]),
            &Vec::from_array(env, [500i128]), &Vec::from_array(env, [token.clone()]),
            &deadline, &options);
        (id, token)
    }

    #[test]
    fn test_dispute_release_and_resolve_release() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let payer = Address::generate(&env);
        let recipient = Address::generate(&env);

        let (id, _) = create_escrow_invoice(&env, &client, &creator, &payer, &recipient, None);

        client.pay(&payer, &id, &500i128);

        let state = client.get_escrow_state(&id).unwrap();
        assert!(!state.disputed);

        client.dispute_release(&id);
        let state = client.get_escrow_state(&id).unwrap();
        assert!(state.disputed);

        client.resolve_dispute(&id, &true);
        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Released);
    }

    #[test]
    fn test_dispute_release_and_resolve_refund() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let payer = Address::generate(&env);
        let recipient = Address::generate(&env);

        let (id, _) = create_escrow_invoice(&env, &client, &creator, &payer, &recipient, None);

        client.pay(&payer, &id, &500i128);
        client.dispute_release(&id);

        client.resolve_dispute(&id, &false);
        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Refunded);
    }

    #[test]
    fn test_arbitrator_resolves_dispute() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        let payer = Address::generate(&env);
        let recipient = Address::generate(&env);

        let (id, _) = create_escrow_invoice(&env, &client, &creator, &payer, &recipient, Some(arbitrator.clone()));

        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.arbitrator, Some(arbitrator.clone()));

        client.pay(&payer, &id, &500i128);
        client.dispute_release(&id);

        client.resolve_dispute(&id, &true);
        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Released);
    }

    #[test]
    #[should_panic]
    fn test_release_escrow_rejects_disputed() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let payer = Address::generate(&env);
        let recipient = Address::generate(&env);

        let (id, _) = create_escrow_invoice(&env, &client, &creator, &payer, &recipient, None);

        client.pay(&payer, &id, &500i128);
        client.dispute_release(&id);

        env.ledger().set_timestamp(env.ledger().timestamp() + 7200);

        client.release_escrow(&id);
    }

    // ---------------------------------------------------------
    // Protocol 26 CAP-78: bump_invoice_ttl
    // Protocol 25/26 crypto: get_invoice_fingerprint
    // Protocol 26 CAP-82: checked arithmetic in stats
    // ---------------------------------------------------------

    #[test]
    fn test_bump_invoice_ttl_succeeds() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [500i128]),
            &Vec::from_array(&env, [token]),
            &deadline,
            &default_options(&env),
        );

        // Should succeed without panic — CAP-78 TTL extension
        client.bump_invoice_ttl(&id);
    }

    #[test]
    fn test_invoice_fingerprint_is_deterministic() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [500i128]),
            &Vec::from_array(&env, [token]),
            &deadline,
            &default_options(&env),
        );

        // Same invoice should produce same fingerprint on repeated calls
        let fp1 = client.get_invoice_fingerprint(&id);
        let fp2 = client.get_invoice_fingerprint(&id);
        assert_eq!(fp1, fp2, "fingerprint must be deterministic");
    }

    #[test]
    fn test_invoice_stats_checked_arithmetic() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [1000i128]),
            &Vec::from_array(&env, [token]),
            &deadline,
            &default_options(&env),
        );

        let stats = client.get_invoice_stats(&id);
        assert_eq!(stats.total, 1000i128);
        assert_eq!(stats.funded, 0i128);
        assert_eq!(stats.completion_bps, 0u32);
    }

    // -----------------------------------------------------------------------
    // preview_payout tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_preview_payout_proportional_two_recipients() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let t1 = Address::generate(&env);
        let t2 = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        // 60/40 split: amounts [600, 400] out of total 1000
        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [r1, r2]),
            &Vec::from_array(&env, [600i128, 400i128]),
            &Vec::from_array(&env, [t1, t2]),
            &deadline,
            &default_options(&env),
        );

        // Preview paying 1000 — should get [600, 400]
        let preview = client.preview_payout(&id, &1000i128);
        assert_eq!(preview.len(), 2);
        assert_eq!(preview.get(0).unwrap(), 600i128);
        assert_eq!(preview.get(1).unwrap(), 400i128);
    }

    #[test]
    fn test_preview_payout_dust_goes_to_last_recipient() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let t1 = Address::generate(&env);
        let t2 = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        // Equal split but odd amount — dust should go to last recipient
        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [r1, r2]),
            &Vec::from_array(&env, [500i128, 500i128]),
            &Vec::from_array(&env, [t1, t2]),
            &deadline,
            &default_options(&env),
        );

        // Preview paying 101 — 50 + 51 (dust to last)
        let preview = client.preview_payout(&id, &101i128);
        assert_eq!(preview.len(), 2);
        let sum: i128 = preview.iter().sum();
        assert_eq!(sum, 101i128, "amounts must sum to payment amount");
    }

    #[test]
    fn test_preview_payout_single_recipient() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [1000i128]),
            &Vec::from_array(&env, [token]),
            &deadline,
            &default_options(&env),
        );

        let preview = client.preview_payout(&id, &500i128);
        assert_eq!(preview.len(), 1);
        assert_eq!(preview.get(0).unwrap(), 500i128);
    }

    #[test]
    fn test_preview_payout_percentage_split_rules() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let t1 = Address::generate(&env);
        let t2 = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let options = InvoiceOptions {
            escrow_enabled: false,
            escrow_release_delay: None,
            split_rules: Vec::from_array(&env, [
                SplitRule::Percentage(7000u32), // 70%
                SplitRule::Percentage(3000u32), // 30%
            ]),
            auto_resolve_rules: Vec::new(&env),
            arbitrator: None,
        };

        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [r1, r2]),
            &Vec::from_array(&env, [700i128, 300i128]),
            &Vec::from_array(&env, [t1, t2]),
            &deadline,
            &options,
        );

        let preview = client.preview_payout(&id, &1000i128);
        assert_eq!(preview.get(0).unwrap(), 700i128); // 70% of 1000
        assert_eq!(preview.get(1).unwrap(), 300i128); // 30% of 1000
    }

    // -----------------------------------------------------------------------
    // get_invoices_by_creator tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_invoices_by_creator_empty_before_create() {
        let (env, client) = setup();
        let creator = Address::generate(&env);

        let ids = client.get_invoices_by_creator(&creator);
        assert_eq!(ids.len(), 0);
    }

    #[test]
    fn test_get_invoices_by_creator_single_invoice() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [1000i128]),
            &Vec::from_array(&env, [token]),
            &deadline,
            &default_options(&env),
        );

        let ids = client.get_invoices_by_creator(&creator);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids.get(0).unwrap(), id);
    }

    #[test]
    fn test_get_invoices_by_creator_multiple_invoices() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id1 = client.create_invoice(&creator, &Vec::from_array(&env, [recipient.clone()]),
            &Vec::from_array(&env, [100i128]), &Vec::from_array(&env, [token.clone()]),
            &deadline, &default_options(&env));
        let id2 = client.create_invoice(&creator, &Vec::from_array(&env, [recipient.clone()]),
            &Vec::from_array(&env, [200i128]), &Vec::from_array(&env, [token.clone()]),
            &deadline, &default_options(&env));
        let id3 = client.create_invoice(&creator, &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [300i128]), &Vec::from_array(&env, [token]),
            &deadline, &default_options(&env));

        let ids = client.get_invoices_by_creator(&creator);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids.get(0).unwrap(), id1);
        assert_eq!(ids.get(1).unwrap(), id2);
        assert_eq!(ids.get(2).unwrap(), id3);
    }

    #[test]
    fn test_get_invoices_by_creator_isolated_per_creator() {
        let (env, client) = setup();
        let creator_a = Address::generate(&env);
        let creator_b = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        client.create_invoice(&creator_a, &Vec::from_array(&env, [recipient.clone()]),
            &Vec::from_array(&env, [100i128]), &Vec::from_array(&env, [token.clone()]),
            &deadline, &default_options(&env));
        client.create_invoice(&creator_b, &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [200i128]), &Vec::from_array(&env, [token]),
            &deadline, &default_options(&env));

        let ids_a = client.get_invoices_by_creator(&creator_a);
        let ids_b = client.get_invoices_by_creator(&creator_b);
        assert_eq!(ids_a.len(), 1);
        assert_eq!(ids_b.len(), 1);
        // They should hold different invoice IDs
        assert_ne!(ids_a.get(0).unwrap(), ids_b.get(0).unwrap());
    }

    #[test]
    fn test_get_invoices_by_creator_includes_batch() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let params = CreateInvoiceParams {
            recipients: Vec::from_array(&env, [recipient]),
            amounts: Vec::from_array(&env, [100i128]),
            tokens: Vec::from_array(&env, [token]),
            deadline,
        };
        let batch_ids = client.create_batch(&creator, &Vec::from_array(&env, [params.clone(), params]));
        let index_ids = client.get_invoices_by_creator(&creator);

        assert_eq!(index_ids.len(), 2);
        assert_eq!(index_ids.get(0).unwrap(), batch_ids.get(0).unwrap());
        assert_eq!(index_ids.get(1).unwrap(), batch_ids.get(1).unwrap());
    }

    #[test]
    fn test_get_invoices_by_creator_includes_recurring() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_recurring(
            &creator,
            &Vec::from_array(&env, [recipient]),
            &Vec::from_array(&env, [500i128]),
            &Vec::from_array(&env, [token]),
            &deadline,
            &(86400u64 * 30),
            &3u32,
        );

        let ids = client.get_invoices_by_creator(&creator);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids.get(0).unwrap(), id);
    }

    // -----------------------------------------------------------------------
    // Fallback balance recovery tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_claimable_balance_returns_zero_initially() {
        let (env, client) = setup();
        let account = Address::generate(&env);
        let token = Address::generate(&env);

        let balance = client.get_claimable_balance(&account, &token);
        assert_eq!(balance, 0i128);
    }

    #[test]
    #[should_panic(expected = "no claimable balance")]
    fn test_claim_with_zero_balance_fails() {
        let (env, client) = setup();
        let account = Address::generate(&env);
        let token = Address::generate(&env);

        // Attempting to claim with no balance should panic
        client.claim(&account, &token);
    }

    #[test]
    fn test_claim_withdraws_credited_balance() {
        let (env, client) = setup();
        let account = Address::generate(&env);
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract(admin.clone());
        let sac = token::StellarAssetClient::new(&env, &token);

        // Mint tokens to the contract (simulating failed transfer funds held in vault)
        let contract_addr = client.address.clone();
        sac.mint(&contract_addr, &1000i128);

        // Manually credit a balance (simulating what would happen on failed transfer)
        // We do this by directly manipulating storage since credit_account is private
        use soroban_sdk::symbol_short;
        let key = (symbol_short!("acc_bal"), account.clone(), token.clone());
        env.as_contract(&contract_addr, || {
            env.storage().persistent().set(&key, &500i128);
            env.storage().persistent().extend_ttl(&key, 100_000, 6_307_200);
        });

        // Verify balance is queryable
        let balance_before = client.get_claimable_balance(&account, &token);
        assert_eq!(balance_before, 500i128);

        // Claim should transfer the balance
        let claimed = client.claim(&account, &token);
        assert_eq!(claimed, 500i128);

        // Balance should now be zero
        let balance_after = client.get_claimable_balance(&account, &token);
        assert_eq!(balance_after, 0i128);

        // Account should have received the tokens
        assert_eq!(sac.balance(&account), 500i128);
    }

    #[test]
    #[should_panic(expected = "no claimable balance")]
    fn test_claim_twice_fails() {
        let (env, client) = setup();
        let account = Address::generate(&env);
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract(admin.clone());
        let sac = token::StellarAssetClient::new(&env, &token);

        let contract_addr = client.address.clone();
        sac.mint(&contract_addr, &1000i128);

        // Credit balance
        use soroban_sdk::symbol_short;
        let key = (symbol_short!("acc_bal"), account.clone(), token.clone());
        env.as_contract(&contract_addr, || {
            env.storage().persistent().set(&key, &500i128);
            env.storage().persistent().extend_ttl(&key, 100_000, 6_307_200);
        });

        // First claim succeeds
        client.claim(&account, &token);

        // Second claim should panic
        client.claim(&account, &token);
    }

    #[test]
    fn test_claimable_balance_accumulation() {
        let (env, client) = setup();
        let account = Address::generate(&env);
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract(admin.clone());
        let sac = token::StellarAssetClient::new(&env, &token);

        let contract_addr = client.address.clone();
        sac.mint(&contract_addr, &2000i128);

        // Simulate multiple failed transfers crediting the same account
        use soroban_sdk::symbol_short;
        let key = (symbol_short!("acc_bal"), account.clone(), token.clone());
        env.as_contract(&contract_addr, || {
            // First failure
            let current1: i128 = env.storage().persistent().get(&key).unwrap_or(0);
            env.storage().persistent().set(&key, &(current1 + 300i128));
            // Second failure
            let current2: i128 = env.storage().persistent().get(&key).unwrap_or(0);
            env.storage().persistent().set(&key, &(current2 + 700i128));
        });

        // Balance should be sum of all failures
        let balance = client.get_claimable_balance(&account, &token);
        assert_eq!(balance, 1000i128);

        // Claim should withdraw the full accumulated amount
        let claimed = client.claim(&account, &token);
        assert_eq!(claimed, 1000i128);
        assert_eq!(sac.balance(&account), 1000i128);
    }

    #[test]
    fn test_claimable_balance_isolated_per_token() {
        let (env, client) = setup();
        let account = Address::generate(&env);
        let admin = Address::generate(&env);
        let token_a = env.register_stellar_asset_contract(admin.clone());
        let token_b = env.register_stellar_asset_contract(admin.clone());

        let contract_addr = client.address.clone();

        // Credit balances for two different tokens
        use soroban_sdk::symbol_short;
        let key_a = (symbol_short!("acc_bal"), account.clone(), token_a.clone());
        let key_b = (symbol_short!("acc_bal"), account.clone(), token_b.clone());
        env.as_contract(&contract_addr, || {
            env.storage().persistent().set(&key_a, &500i128);
            env.storage().persistent().set(&key_b, &300i128);
        });

        // Balances should be independent
        assert_eq!(client.get_claimable_balance(&account, &token_a), 500i128);
        assert_eq!(client.get_claimable_balance(&account, &token_b), 300i128);
    }

    #[test]
    fn test_get_invoice_count_increments() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract(admin.clone());
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);

        assert_eq!(client.get_invoice_count(), 0u64);

        let deadline = env.ledger().timestamp() + 86400;
        let options = default_options(&env);

        client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &options,
        );
        assert_eq!(client.get_invoice_count(), 1u64);

        client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 500i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &options,
        );
        assert_eq!(client.get_invoice_count(), 2u64);
    }

    #[test]
    fn test_get_invoices_by_payer() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract(admin.clone());
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let payer = Address::generate(&env);

        let sac = token::StellarAssetClient::new(&env, &token);
        sac.mint(&payer, &5000i128);

        let deadline = env.ledger().timestamp() + 86400;
        let options = default_options(&env);

        let id1 = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &options,
        );
        let id2 = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 500i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &options,
        );

        client.pay(&payer, &id1, &1000i128);
        client.pay(&payer, &id2, &500i128);

        let paid = client.get_invoices_by_payer(&payer);
        assert_eq!(paid.len(), 2);
        assert!(paid.contains(&id1));
        assert!(paid.contains(&id2));
    }

    #[test]
    fn test_get_invoices_by_payer_deduplicates() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract(admin.clone());
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let payer = Address::generate(&env);

        let sac = token::StellarAssetClient::new(&env, &token);
        sac.mint(&payer, &5000i128);

        let deadline = env.ledger().timestamp() + 86400;
        let options = default_options(&env);

        let id1 = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 2000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &options,
        );

        // Pay same invoice twice — payer index should deduplicate
        client.pay(&payer, &id1, &1000i128);
        client.pay(&payer, &id1, &1000i128);

        let paid = client.get_invoices_by_payer(&payer);
        assert_eq!(paid.len(), 1, "same invoice paid twice should only appear once in payer index");
    }

    #[test]
    #[should_panic(expected = "payment amount must be positive")]
    fn test_zero_amount_payment_fails() {
        let (env, client) = setup();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;
        let id = client.create_invoice(&creator, &Vec::from_array(&env, [recipient]), &Vec::from_array(&env, [1000i128]), &Vec::from_array(&env, [token.clone()]), &deadline, &default_options(&env));
        client.pay(&creator, &id, &0i128); // Should fail
    }
}

#[cfg(test)]
mod test_get_invoices_by_creator {
    use soroban_sdk::{testutils::Address as _, token, Address, Env};
    use soroban_sdk::testutils::Ledger as _;
    use crate::{
        types::{InvoiceOptions, InvoiceStatus},
        SharpyContractClient,
    };

    fn setup() -> (Env, SharpyContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::SharpyContract, ());
        let client = SharpyContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &treasury);
        (env, client)
    }

    fn default_options(env: &Env) -> InvoiceOptions {
        InvoiceOptions {
            escrow_enabled: false,
            escrow_release_delay: None,
            split_rules: soroban_sdk::vec![env],
            auto_resolve_rules: soroban_sdk::vec![env],
            arbitrator: None,
        }
    }

    #[test]
    fn test_creator_index_populated_on_create() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract(admin);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id1 = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &default_options(&env),
        );
        let id2 = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 2000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &default_options(&env),
        );

        let by_creator = client.get_invoices_by_creator(&creator);
        assert_eq!(by_creator.len(), 2, "creator index should have 2 entries");
        assert!(by_creator.contains(&id1));
        assert!(by_creator.contains(&id2));
    }

    #[test]
    fn test_creator_index_empty_for_new_address() {
        let (env, client) = setup();
        let stranger = Address::generate(&env);
        let by_creator = client.get_invoices_by_creator(&stranger);
        assert_eq!(by_creator.len(), 0, "unknown creator should return empty vec");
    }

    #[test]
    fn test_creator_index_does_not_cross_contaminate() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract(admin);
        let creator_a = Address::generate(&env);
        let creator_b = Address::generate(&env);
        let recipient = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        client.create_invoice(
            &creator_a,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 500i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &default_options(&env),
        );
        client.create_invoice(
            &creator_b,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 500i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &default_options(&env),
        );

        assert_eq!(client.get_invoices_by_creator(&creator_a).len(), 1);
        assert_eq!(client.get_invoices_by_creator(&creator_b).len(), 1);
    }
}

#[cfg(test)]
mod test_preview_payout {
    use soroban_sdk::{testutils::Address as _, Address, Env};
    use soroban_sdk::testutils::Ledger as _;
    use crate::{
        types::{InvoiceOptions, SplitRule},
        SharpyContractClient,
    };

    fn setup() -> (Env, SharpyContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::SharpyContract, ());
        let client = SharpyContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &treasury);
        (env, client)
    }

    fn opts_with_rules(env: &Env, rules: soroban_sdk::Vec<SplitRule>) -> InvoiceOptions {
        InvoiceOptions {
            escrow_enabled: false,
            escrow_release_delay: None,
            split_rules: rules,
            auto_resolve_rules: soroban_sdk::vec![env],
            arbitrator: None,
        }
    }

    fn no_rules(env: &Env) -> InvoiceOptions {
        opts_with_rules(env, soroban_sdk::vec![env])
    }

    #[test]
    fn test_preview_payout_proportional_two_recipients() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, r1.clone(), r2.clone()],
            &soroban_sdk::vec![&env, 6000i128, 4000i128], // 60/40 split
            &soroban_sdk::vec![&env, token.clone(), token.clone()],
            &deadline,
            &no_rules(&env),
        );

        let payouts = client.preview_payout(&id, &10_000i128);
        assert_eq!(payouts.len(), 2);
        assert_eq!(payouts.get(0).unwrap(), 6000i128);
        assert_eq!(payouts.get(1).unwrap(), 4000i128);
    }

    #[test]
    fn test_preview_payout_percentage_split_rule() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;
        let rules = soroban_sdk::vec![&env, SplitRule::Percentage(7000u32), SplitRule::Percentage(3000u32)];

        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, r1.clone(), r2.clone()],
            &soroban_sdk::vec![&env, 7000i128, 3000i128],
            &soroban_sdk::vec![&env, token.clone(), token.clone()],
            &deadline,
            &opts_with_rules(&env, rules),
        );

        let payouts = client.preview_payout(&id, &10_000i128);
        assert_eq!(payouts.get(0).unwrap(), 7000i128); // 70% of 10_000
        assert_eq!(payouts.get(1).unwrap(), 3000i128); // 30% of 10_000
    }

    #[test]
    fn test_preview_payout_fixed_split_rule() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;
        let rules = soroban_sdk::vec![&env, SplitRule::Fixed(500i128)];

        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, r1.clone()],
            &soroban_sdk::vec![&env, 500i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &opts_with_rules(&env, rules),
        );

        let payouts = client.preview_payout(&id, &1000i128);
        assert_eq!(payouts.get(0).unwrap(), 500i128); // Always fixed 500
    }

    #[test]
    fn test_preview_payout_last_recipient_gets_dust() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, r1.clone(), r2.clone()],
            &soroban_sdk::vec![&env, 1i128, 1i128], // equal split of odd amount
            &soroban_sdk::vec![&env, token.clone(), token.clone()],
            &deadline,
            &no_rules(&env),
        );

        let payouts = client.preview_payout(&id, &3i128); // 3 stroop — last gets remainder
        let sum: i128 = payouts.iter().sum();
        assert_eq!(sum, 3i128, "all funds must be distributed — no dust left behind");
    }
}

#[cfg(test)]
mod test_invoice_count_and_cancel_audit {
    use soroban_sdk::{testutils::Address as _, symbol_short, Address, Env};
    use soroban_sdk::testutils::Ledger as _;
    use crate::{
        types::{InvoiceOptions, InvoiceStatus},
        SharpyContractClient,
    };

    fn setup() -> (Env, SharpyContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::SharpyContract, ());
        let client = SharpyContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &treasury);
        (env, client)
    }

    fn no_rules(env: &Env) -> InvoiceOptions {
        InvoiceOptions {
            escrow_enabled: false,
            escrow_release_delay: None,
            split_rules: soroban_sdk::vec![env],
            auto_resolve_rules: soroban_sdk::vec![env],
            arbitrator: None,
        }
    }

    #[test]
    fn test_get_invoice_count_increments() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        assert_eq!(client.get_invoice_count(), 0u64);

        client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &no_rules(&env),
        );
        assert_eq!(client.get_invoice_count(), 1u64);

        client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 2000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &no_rules(&env),
        );
        assert_eq!(client.get_invoice_count(), 2u64);
    }

    #[test]
    fn test_cancel_invoice_writes_audit_entry() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &no_rules(&env),
        );

        client.cancel_invoice(&creator, &id);

        let log = client.get_audit_log(&id);
        let mut found_cancel = false;
        for entry in log.iter() {
            if entry.action == symbol_short!("cancel") {
                found_cancel = true;
                break;
            }
        }
        assert!(found_cancel, "audit log must contain 'cancel' entry after cancel_invoice");
    }

    #[test]
    fn test_cancel_invoice_with_no_payments_sets_cancelled_status() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &no_rules(&env),
        );

        client.cancel_invoice(&creator, &id);
        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Cancelled, "unfunded cancellation should be Cancelled, not Refunded");
    }
}

#[cfg(test)]
mod test_pause_circuit_breaker {
    use soroban_sdk::{testutils::Address as _, Address, Env};
    use soroban_sdk::testutils::Ledger as _;
    use crate::{types::InvoiceOptions, SharpyContractClient};

    fn setup() -> (Env, SharpyContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::SharpyContract, ());
        let client = SharpyContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &treasury);
        (env, client, admin)
    }

    fn no_rules(env: &Env) -> InvoiceOptions {
        InvoiceOptions {
            escrow_enabled: false,
            escrow_release_delay: None,
            split_rules: soroban_sdk::vec![env],
            auto_resolve_rules: soroban_sdk::vec![env],
            arbitrator: None,
        }
    }

    #[test]
    #[should_panic(expected = "contract is paused")]
    fn test_create_invoice_blocked_when_paused() {
        let (env, client, _admin) = setup();
        client.pause();
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;
        client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token],
            &deadline,
            &no_rules(&env),
        );
    }

    #[test]
    fn test_unpause_restores_functionality() {
        let (env, client, _admin) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        client.pause();
        client.unpause();

        // Should succeed after unpause
        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token],
            &deadline,
            &no_rules(&env),
        );
        assert!(id > 0, "create_invoice should succeed after unpause");
    }

    #[test]
    #[should_panic(expected = "contract is paused")]
    fn test_pay_blocked_when_paused() {
        let (env, client, _admin) = setup();
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract(admin.clone());
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let payer = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        sac.mint(&payer, &5000i128);

        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token],
            &deadline,
            &no_rules(&env),
        );

        client.pause();
        client.pay(&payer, &id, &1000i128); // Should panic
    }
}

#[cfg(test)]
mod test_batch_creator_index {
    use soroban_sdk::{testutils::Address as _, Address, Env};
    use soroban_sdk::testutils::Ledger as _;
    use crate::{types::CreateInvoiceParams, SharpyContractClient};

    fn setup() -> (Env, SharpyContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::SharpyContract, ());
        let client = SharpyContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &treasury);
        (env, client)
    }

    #[test]
    fn test_batch_invoices_indexed_for_creator() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let batch = soroban_sdk::vec![
            &env,
            CreateInvoiceParams {
                recipients: soroban_sdk::vec![&env, recipient.clone()],
                amounts: soroban_sdk::vec![&env, 1000i128],
                tokens: soroban_sdk::vec![&env, token.clone()],
                deadline,
            },
            CreateInvoiceParams {
                recipients: soroban_sdk::vec![&env, recipient.clone()],
                amounts: soroban_sdk::vec![&env, 2000i128],
                tokens: soroban_sdk::vec![&env, token.clone()],
                deadline,
            },
            CreateInvoiceParams {
                recipients: soroban_sdk::vec![&env, recipient.clone()],
                amounts: soroban_sdk::vec![&env, 3000i128],
                tokens: soroban_sdk::vec![&env, token.clone()],
                deadline,
            },
        ];

        let ids = client.create_batch(&creator, &batch);
        assert_eq!(ids.len(), 3, "batch should return 3 IDs");

        let by_creator = client.get_invoices_by_creator(&creator);
        assert_eq!(by_creator.len(), 3, "all 3 batch invoices should appear in creator index");

        for id in ids.iter() {
            assert!(by_creator.contains(&id), "batch invoice ID {} not found in creator index", id);
        }
    }

    #[test]
    fn test_batch_invoice_count_increments_correctly() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        assert_eq!(client.get_invoice_count(), 0u64);

        let batch = soroban_sdk::vec![
            &env,
            CreateInvoiceParams {
                recipients: soroban_sdk::vec![&env, recipient.clone()],
                amounts: soroban_sdk::vec![&env, 500i128],
                tokens: soroban_sdk::vec![&env, token.clone()],
                deadline,
            },
            CreateInvoiceParams {
                recipients: soroban_sdk::vec![&env, recipient.clone()],
                amounts: soroban_sdk::vec![&env, 500i128],
                tokens: soroban_sdk::vec![&env, token.clone()],
                deadline,
            },
        ];

        client.create_batch(&creator, &batch);
        assert_eq!(client.get_invoice_count(), 2u64, "invoice counter should reflect all batch items");
    }
}

#[cfg(test)]
mod test_tiered_split {
    use soroban_sdk::{testutils::Address as _, token, Address, Env};
    use soroban_sdk::testutils::Ledger as _;
    use crate::{types::{InvoiceOptions, SplitRule}, SharpyContractClient};

    fn setup() -> (Env, SharpyContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::SharpyContract, ());
        let client = SharpyContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &treasury);
        (env, client)
    }

    #[test]
    fn test_tiered_split_below_threshold_pays_zero() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;
        // Threshold = 10_000, so below that recipient gets 0
        let rules = soroban_sdk::vec![&env, SplitRule::Tiered(10_000i128, 5000u32)];
        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, r1.clone()],
            &soroban_sdk::vec![&env, 5000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &InvoiceOptions {
                escrow_enabled: false,
                escrow_release_delay: None,
                split_rules: rules,
                auto_resolve_rules: soroban_sdk::vec![&env],
                arbitrator: None,
            },
        );
        // Amount 5000 < threshold 10_000 — preview should return 0
        let payouts = client.preview_payout(&id, &5000i128);
        assert_eq!(payouts.get(0).unwrap(), 0i128, "below threshold, tiered payout must be 0");
    }

    #[test]
    fn test_tiered_split_above_threshold_pays_percentage() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;
        // Threshold = 5_000, bps = 5000 (50%)
        let rules = soroban_sdk::vec![&env, SplitRule::Tiered(5_000i128, 5000u32)];
        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, r1.clone()],
            &soroban_sdk::vec![&env, 10_000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &InvoiceOptions {
                escrow_enabled: false,
                escrow_release_delay: None,
                split_rules: rules,
                auto_resolve_rules: soroban_sdk::vec![&env],
                arbitrator: None,
            },
        );
        // Amount 10_000 > threshold 5_000 — 50% = 5_000
        let payouts = client.preview_payout(&id, &10_000i128);
        assert_eq!(payouts.get(0).unwrap(), 5_000i128, "above threshold, tiered payout must be 50% of funded");
    }
}

#[cfg(test)]
mod test_recurring_chain {
    use soroban_sdk::{testutils::Address as _, token, Address, Env};
    use soroban_sdk::testutils::Ledger as _;
    use crate::{types::InvoiceStatus, SharpyContractClient};

    fn setup() -> (Env, SharpyContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::SharpyContract, ());
        let client = SharpyContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &treasury);
        (env, client)
    }

    #[test]
    fn test_recurring_release_spawns_next_invoice() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let tok = env.register_stellar_asset_contract(admin.clone());
        let sac = token::StellarAssetClient::new(&env, &tok);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let payer = Address::generate(&env);

        sac.mint(&payer, &5000i128);

        let deadline = env.ledger().timestamp() + 86400;
        let interval = 86400u64;

        let id = client.create_recurring(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, tok.clone()],
            &deadline,
            &interval,
            &3u32,
        );

        client.pay(&payer, &id, &1000i128);

        let next_id = client.get_next_recurring(&id);
        assert!(next_id.is_some(), "releasing recurring invoice should spawn next invoice");

        let next = client.get_invoice(&next_id.unwrap());
        assert_eq!(next.status, InvoiceStatus::Pending, "next recurring invoice should be Pending");
        assert_eq!(next.amounts.get(0).unwrap(), 1000i128, "next invoice should have same amount");
    }

    #[test]
    fn test_recurring_stops_at_max_recurrences() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let tok = env.register_stellar_asset_contract(admin.clone());
        let sac = token::StellarAssetClient::new(&env, &tok);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let payer = Address::generate(&env);

        sac.mint(&payer, &10_000i128);

        let deadline = env.ledger().timestamp() + 86400;

        // max_recurrences = 1 means only the first invoice — no next one should spawn
        let id = client.create_recurring(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, tok.clone()],
            &deadline,
            &86400u64,
            &1u32,
        );

        client.pay(&payer, &id, &1000i128);

        let next_id = client.get_next_recurring(&id);
        assert!(next_id.is_none(), "max_recurrences=1 means no next invoice should be created");
    }
}

#[cfg(test)]
mod test_payer_total_and_fingerprint {
    use soroban_sdk::{testutils::Address as _, token, Address, Env};
    use soroban_sdk::testutils::Ledger as _;
    use crate::{types::{InvoiceOptions, SplitRule}, SharpyContractClient};

    fn setup() -> (Env, SharpyContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::SharpyContract, ());
        let client = SharpyContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &treasury);
        (env, client)
    }

    fn no_rules(env: &Env) -> InvoiceOptions {
        InvoiceOptions {
            escrow_enabled: false,
            escrow_release_delay: None,
            split_rules: soroban_sdk::vec![env],
            auto_resolve_rules: soroban_sdk::vec![env],
            arbitrator: None,
        }
    }

    #[test]
    fn test_get_payer_total_aggregates_multiple_payments() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let tok = env.register_stellar_asset_contract(admin.clone());
        let sac = token::StellarAssetClient::new(&env, &tok);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let payer = Address::generate(&env);

        sac.mint(&payer, &5000i128);

        let deadline = env.ledger().timestamp() + 86400;
        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient],
            &soroban_sdk::vec![&env, 3000i128],
            &soroban_sdk::vec![&env, tok.clone()],
            &deadline,
            &no_rules(&env),
        );

        client.pay(&payer, &id, &1000i128);
        client.pay(&payer, &id, &1000i128);

        let total = client.get_payer_total(&id, &payer);
        assert_eq!(total, 2000i128, "payer total should sum all payments from that address");
    }

    #[test]
    fn test_get_payer_total_zero_for_non_payer() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let stranger = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token],
            &deadline,
            &no_rules(&env),
        );

        assert_eq!(client.get_payer_total(&id, &stranger), 0i128, "non-payer should have 0 total");
    }

    #[test]
    fn test_invoice_fingerprint_is_deterministic() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token],
            &deadline,
            &no_rules(&env),
        );

        let fp1 = client.get_invoice_fingerprint(&id);
        let fp2 = client.get_invoice_fingerprint(&id);
        assert_eq!(fp1, fp2, "fingerprint must be deterministic — same call returns same hash");
    }

    #[test]
    fn test_invoice_fingerprint_differs_between_invoices() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        let id1 = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 1000i128],
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &no_rules(&env),
        );
        let id2 = client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, recipient.clone()],
            &soroban_sdk::vec![&env, 2000i128], // different amount → different hash
            &soroban_sdk::vec![&env, token.clone()],
            &deadline,
            &no_rules(&env),
        );

        let fp1 = client.get_invoice_fingerprint(&id1);
        let fp2 = client.get_invoice_fingerprint(&id2);
        assert_ne!(fp1, fp2, "fingerprints must differ between invoices with different amounts");
    }

    #[test]
    #[should_panic(expected = "split rules exceed 100%")]
    fn test_percentage_split_rules_over_100_bps_panics() {
        let (env, client) = setup();
        let token = Address::generate(&env);
        let creator = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let deadline = env.ledger().timestamp() + 86400;

        // 7000 + 5000 = 12000 bps > 10000 — should panic
        let rules = soroban_sdk::vec![
            &env,
            SplitRule::Percentage(7000u32),
            SplitRule::Percentage(5000u32),
        ];
        client.create_invoice(
            &creator,
            &soroban_sdk::vec![&env, r1, r2],
            &soroban_sdk::vec![&env, 700i128, 500i128],
            &soroban_sdk::vec![&env, token.clone(), token.clone()],
            &deadline,
            &InvoiceOptions {
                escrow_enabled: false,
                escrow_release_delay: None,
                split_rules: rules,
                auto_resolve_rules: soroban_sdk::vec![&env],
                arbitrator: None,
            },
        );
    }

    #[test]
    #[should_panic(expected = "payments must not be empty")]
    fn test_pool_pay_empty_vec_panics() {
        let (env, client) = setup();
        let payer = Address::generate(&env);
        client.pool_pay(&payer, &soroban_sdk::vec![&env]);
    }
}
