use soroban_sdk::{contract, contracttype, symbol_short, Address, Env, Symbol, Vec};

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
    let event = InvoiceCreatedEvent {
        id,
        creator: creator.clone(),
    };
    env.events().publish((symbol_short!("created"),), event);
}

pub fn payment_received(env: &Env, invoice_id: u64, payer: &Address, amount: i128) {
    let event = PaymentReceivedEvent {
        invoice_id,
        payer: payer.clone(),
        amount,
    };
    env.events().publish((symbol_short!("payment"),), event);
}

pub fn invoice_released(env: &Env, id: u64, recipients: &Vec<Address>) {
    let event = InvoiceReleasedEvent { id };
    env.events().publish((symbol_short!("released"),), event);
}

pub fn invoice_refunded(env: &Env, id: u64) {
    let event = InvoiceRefundedEvent { id };
    env.events().publish((symbol_short!("refunded"),), event);
}

pub fn payer_refunded(env: &Env, invoice_id: u64, payer: &Address, amount: i128) {
    let event = PayerRefundedEvent {
        invoice_id,
        payer: payer.clone(),
        amount,
    };
    env.events().publish((symbol_short!("payer_refund"),), event);
}
