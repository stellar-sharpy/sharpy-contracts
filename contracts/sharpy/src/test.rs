#[cfg(test)]
mod tests {
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, Vec};

    use crate::{
        types::{CreateInvoiceParams, InvoiceOptions, InvoiceStatus, SplitRule, ResolveRule, ResolveAction},
        SharpyContract, SharpyContractClient,
    };

    #[test]
    fn test_create_invoice() {
        let env = Env::default();
        let admin = Address::random(&env);
        let creator = Address::random(&env);
        let recipient = Address::random(&env);
        let treasury = Address::random(&env);
        let token = Address::random(&env);

        let contract = SharpyContractClient::new(&env, &Address::random(&env));
        contract.initialize(&admin, &treasury);

        let recipients = Vec::from_array(&env, [recipient.clone()]);
        let amounts = Vec::from_array(&env, [1000i128]);
        let deadline = env.ledger().timestamp() + 86400; // 1 day

        let options = InvoiceOptions {
            escrow_enabled: false,
            escrow_release_delay: None,
            split_rules: Vec::new(&env),
            auto_resolve_rules: Vec::new(&env),
        };

        let invoice_id = contract.create_invoice(
            &creator,
            &recipients,
            &amounts,
            &token,
            &deadline,
            &options,
        );

        assert!(invoice_id > 0);

        let invoice = contract.get_invoice(&invoice_id);
        assert_eq!(invoice.status, InvoiceStatus::Pending);
        assert_eq!(invoice.funded, 0);
    }

    #[test]
    fn test_batch_create() {
        let env = Env::default();
        let admin = Address::random(&env);
        let creator = Address::random(&env);
        let recipient = Address::random(&env);
        let treasury = Address::random(&env);
        let token = Address::random(&env);

        let contract = SharpyContractClient::new(&env, &Address::random(&env));
        contract.initialize(&admin, &treasury);

        let recipients = Vec::from_array(&env, [recipient.clone()]);
        let amounts = Vec::from_array(&env, [500i128]);
        let deadline = env.ledger().timestamp() + 86400;

        let params1 = CreateInvoiceParams {
            recipients: recipients.clone(),
            amounts: amounts.clone(),
            token: token.clone(),
            deadline,
        };

        let params2 = CreateInvoiceParams {
            recipients,
            amounts,
            token,
            deadline,
        };

        let batch = Vec::from_array(&env, [params1, params2]);
        let ids = contract.create_batch(&creator, &batch);

        assert_eq!(ids.len(), 2);
    }
}
