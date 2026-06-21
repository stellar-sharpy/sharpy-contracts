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
    pub funded: i128,
    pub recipient_count: u32,
    pub creator: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct InvoiceRefundedEvent {
    pub id: u64,
    pub funded: i128,
    pub recipient_count: u32,
    pub creator: Address,
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

pub fn invoice_released(env: &Env, id: u64, funded: i128, recipient_count: u32, creator: &Address) {
    env.events().publish((symbol_short!("released"),), InvoiceReleasedEvent { id, funded, recipient_count, creator: creator.clone() });
}

pub fn invoice_refunded(env: &Env, id: u64, funded: i128, recipient_count: u32, creator: &Address) {
    env.events().publish((symbol_short!("refunded"),), InvoiceRefundedEvent { id, funded, recipient_count, creator: creator.clone() });
}

pub fn payer_refunded(env: &Env, invoice_id: u64, payer: &Address, amount: i128) {
    env.events().publish((symbol_short!("pyr"),), PayerRefundedEvent { invoice_id, payer: payer.clone(), amount });
}

#[contracttype]
#[derive(Clone)]
pub struct DisputeRaisedEvent {
    pub invoice_id: u64,
    pub creator: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct DisputeResolvedEvent {
    pub invoice_id: u64,
    pub resolver: Address,
    pub release: bool,
}

pub fn dispute_raised(env: &Env, invoice_id: u64, creator: &Address) {
    env.events().publish((symbol_short!("dispute"),), DisputeRaisedEvent { invoice_id, creator: creator.clone() });
}

pub fn dispute_resolved(env: &Env, invoice_id: u64, resolver: &Address, release: bool) {
    env.events().publish((symbol_short!("dsprslv"),), DisputeResolvedEvent { invoice_id, resolver: resolver.clone(), release });
}
