use soroban_sdk::{contracttype, symbol_short, Address, Env, Vec};

#[contracttype]
#[derive(Clone)]
pub struct InvoiceCreatedEvent {
    pub id: u64,
    pub creator: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct PaymentReceivedEvent {
    pub invoice_id: u64,
    pub payer: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct InvoiceReleasedEvent {
    pub id: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct InvoiceRefundedEvent {
    pub id: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct PayerRefundedEvent {
    pub invoice_id: u64,
    pub payer: Address,
    pub amount: i128,
}

pub fn invoice_created(env: &Env, id: u64, creator: &Address) {
    env.events().publish((symbol_short!("created"),), InvoiceCreatedEvent { id, creator: creator.clone() });
}

pub fn payment_received(env: &Env, invoice_id: u64, payer: &Address, amount: i128) {
    env.events().publish((symbol_short!("payment"),), PaymentReceivedEvent { invoice_id, payer: payer.clone(), amount });
}

pub fn invoice_released(env: &Env, id: u64, _recipients: &Vec<Address>) {
    env.events().publish((symbol_short!("released"),), InvoiceReleasedEvent { id });
}

pub fn invoice_refunded(env: &Env, id: u64) {
    env.events().publish((symbol_short!("refunded"),), InvoiceRefundedEvent { id });
}

pub fn payer_refunded(env: &Env, invoice_id: u64, payer: &Address, amount: i128) {
    env.events().publish((symbol_short!("pyr"),), PayerRefundedEvent { invoice_id, payer: payer.clone(), amount });
}
