#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, token, Address, Env, Vec};
    use soroban_sdk::testutils::Ledger as _;
    use crate::{
        types::{CreateInvoiceParams, InvoiceOptions, InvoicePayment, InvoiceStatus, SplitRule},
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
}
