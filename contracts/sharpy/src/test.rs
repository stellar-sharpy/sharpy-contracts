#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, Address, Env, Vec};
    use crate::{
        types::{CreateInvoiceParams, InvoiceOptions, InvoiceStatus},
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
        }
    }

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
            &token,
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
            token: token.clone(),
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
            &token,
            &deadline,
            &default_options(&env),
        );

        client.cancel_invoice(&creator, &id);
        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Cancelled);
    }
}
