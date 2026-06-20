use soroban_sdk::{contracttype, Address, Symbol, Vec};

/// Split rule for a single recipient — evaluated at release time.
#[contracttype]
#[derive(Clone, Debug)]
pub enum SplitRule {
    /// Pay this exact amount regardless of funded total.
    Fixed(i128),
    /// Pay `funded * bps / 10_000` to the recipient.
    Percentage(u32),
    /// Pay `funded * bps / 10_000` only when `funded > threshold`; else 0.
    /// Encoded as (threshold, bps).
    Tiered(i128, u32),
}

/// Action taken by an auto-resolve rule.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ResolveAction {
    Release,
    Refund,
}

/// Auto-resolve rule — if funded/total >= min_funded_bps/10_000, execute action.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ResolveRule {
    pub min_funded_bps: u32,
    pub action: ResolveAction,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum InvoiceStatus {
    Pending,
    Released,
    Refunded,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Payment {
    pub payer: Address,
    pub amount: i128,
    pub tip: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct AuditEntry {
    pub action: Symbol,
    pub actor: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SubscriptionParams {
    pub creator: Address,
    pub recipients: Vec<Address>,
    pub amounts: Vec<i128>,
    pub tokens: Vec<Address>,
    pub recurrence_interval: u64,
    pub max_recurrences: u32,
    pub num_created: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct InvoicePayment {
    pub invoice_id: u64,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct DisputeState {
    pub release_at: u64,
    pub disputed: bool,
    pub disputed_at: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct InvoiceOptions {
    pub escrow_enabled: bool,
    pub escrow_release_delay: Option<u64>,
    pub split_rules: Vec<SplitRule>,
    pub auto_resolve_rules: Vec<ResolveRule>,
    pub arbitrator: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct CreateInvoiceParams {
    pub recipients: Vec<Address>,
    pub amounts: Vec<i128>,
    pub tokens: Vec<Address>,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Invoice {
    pub version: u32,
    pub creator: Address,
    pub recipients: Vec<Address>,
    pub amounts: Vec<i128>,
    pub tokens: Vec<Address>,
    pub deadline: u64,
    pub funded: i128,
    pub status: InvoiceStatus,
    pub payments: Vec<Payment>,
    pub claimed: Vec<i128>,
    pub frozen: bool,
    pub completion_time: Option<u64>,
    pub escrow_enabled: bool,
    pub escrow_release_delay: u64,
    pub split_rules: Vec<SplitRule>,
    pub auto_resolve_rules: Vec<ResolveRule>,
    pub arbitrator: Option<Address>,
}
