use soroban_sdk::{contracttype, Address, Symbol, Vec};

/// Determines how a recipient's payout is calculated at release time.
/// Evaluated per-recipient — each index in `split_rules` maps to the same index in `recipients`.
#[contracttype]
#[derive(Clone, Debug)]
pub enum SplitRule {
    /// Pay this exact fixed amount regardless of the total funded.
    /// Useful for guaranteed flat fees (e.g. platform fee of 50 USDC).
    Fixed(i128),

    /// Pay `funded * bps / 10_000` to the recipient.
    /// `bps` is basis points: 10_000 = 100%, 5_000 = 50%, 100 = 1%.
    /// Total bps across all Percentage rules must not exceed 10_000.
    Percentage(u32),

    /// Pay `funded * bps / 10_000` only when `funded > threshold`; else pay 0.
    /// Useful for milestone bonuses or conditional splits.
    /// Encoded as `(threshold_amount, bps)`.
    Tiered(i128, u32),
}

/// Action for an auto-resolve rule — executed automatically when the funded threshold is met.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ResolveAction {
    /// Immediately release funds to all recipients.
    Release,
    /// Refund all payers.
    Refund,
}

/// An auto-resolve rule evaluated after each payment.
/// If `funded / total >= min_funded_bps / 10_000`, the action is triggered automatically.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ResolveRule {
    /// Minimum funded percentage (in basis points) to trigger this rule.
    pub min_funded_bps: u32,
    /// Action to take when the threshold is met.
    pub action: ResolveAction,
}

/// Lifecycle state of an invoice.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum InvoiceStatus {
    /// Invoice is open and accepting payments.
    Pending,
    /// Funds have been distributed to all recipients.
    Released,
    /// All payments have been returned to payers (deadline passed or dispute resolved to refund).
    Refunded,
    /// Creator cancelled the invoice before release (no payments were received).
    Cancelled,
}

/// A single payment toward an invoice.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Payment {
    /// Address that made the payment.
    pub payer: Address,
    /// Amount transferred (in token's smallest unit).
    pub amount: i128,
    /// Optional gratuity on top of the invoice amount. Currently always 0.
    pub tip: i128,
}

/// An immutable on-chain audit log entry appended on every state transition.
/// Used for off-chain accountability and dispute evidence.
#[contracttype]
#[derive(Clone, Debug)]
pub struct AuditEntry {
    /// Short symbol identifying the action (e.g. "pay", "release", "cancel", "dispute").
    pub action: Symbol,
    /// Address that triggered the action.
    pub actor: Address,
    /// Ledger timestamp (seconds since Unix epoch) when the action occurred.
    pub timestamp: u64,
}

/// Configuration for a recurring (subscription) invoice chain.
/// Stored per-invoice and copied to the next invoice when the current one is released.
#[contracttype]
#[derive(Clone, Debug)]
pub struct SubscriptionParams {
    /// Original invoice creator — carries over to each generated invoice.
    pub creator: Address,
    /// Recipients in each recurring invoice.
    pub recipients: Vec<Address>,
    /// Amounts per recipient in each recurring invoice.
    pub amounts: Vec<i128>,
    /// Tokens per recipient.
    pub tokens: Vec<Address>,
    /// Seconds between invoice deadlines.
    pub recurrence_interval: u64,
    /// Maximum number of invoices to generate (0 = unlimited).
    pub max_recurrences: u32,
    /// Number of invoices created so far in this chain (starts at 1).
    pub num_created: u32,
}

/// Represents a single payment entry within a `pool_pay` call.
#[contracttype]
#[derive(Clone, Debug)]
pub struct InvoicePayment {
    /// Target invoice ID.
    pub invoice_id: u64,
    /// Amount to pay toward this invoice.
    pub amount: i128,
}

/// Escrow state stored when a fully-funded escrow invoice enters hold.
/// Removed when the invoice is released or refunded.
#[contracttype]
#[derive(Clone, Debug)]
pub struct DisputeState {
    /// Ledger timestamp after which the escrow can be released (no dispute).
    pub release_at: u64,
    /// Whether a dispute has been raised by the creator.
    pub disputed: bool,
    /// Ledger timestamp when the dispute was raised (0 if none).
    pub disputed_at: u64,
}

/// Options passed to `create_invoice` to configure escrow, split rules, and arbitration.
#[contracttype]
#[derive(Clone, Debug)]
pub struct InvoiceOptions {
    /// Enable escrow hold — funds are locked until `release_at` or arbitrator resolves.
    pub escrow_enabled: bool,
    /// Seconds to hold funds in escrow after full payment. Required if `escrow_enabled` is true.
    pub escrow_release_delay: Option<u64>,
    /// Per-recipient split rules. Must match `recipients` length if non-empty.
    pub split_rules: Vec<SplitRule>,
    /// Auto-resolve rules evaluated after each payment. Currently unused in release logic.
    pub auto_resolve_rules: Vec<ResolveRule>,
    /// Optional arbitrator address for dispute resolution. Falls back to creator if None.
    pub arbitrator: Option<Address>,
}

/// Full invoice state stored on-chain.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Invoice {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Address that created the invoice.
    pub creator: Address,
    /// Ordered list of recipient addresses.
    pub recipients: Vec<Address>,
    /// Target amount per recipient (in token's smallest unit).
    pub amounts: Vec<i128>,
    /// Token contract address per recipient (enables multi-token invoices).
    pub tokens: Vec<Address>,
    /// Unix timestamp after which the invoice can be refunded.
    pub deadline: u64,
    /// Total amount funded so far.
    pub funded: i128,
    /// Current lifecycle status.
    pub status: InvoiceStatus,
    /// All individual payments made toward this invoice.
    pub payments: Vec<Payment>,
    /// Amount already claimed per recipient (index mirrors `recipients`). Currently unused.
    pub claimed: Vec<i128>,
    /// Reserved: future freeze mechanism. Currently always false.
    pub frozen: bool,
    /// Ledger timestamp when the invoice reached a terminal state (Released/Refunded/Cancelled).
    pub completion_time: Option<u64>,
    /// Whether escrow hold is enabled.
    pub escrow_enabled: bool,
    /// Seconds to hold in escrow after full payment. Zero if escrow is disabled.
    pub escrow_release_delay: u64,
    /// Per-recipient split rules (empty = proportional distribution).
    pub split_rules: Vec<SplitRule>,
    /// Auto-resolve rules (currently stored but not evaluated in release logic).
    pub auto_resolve_rules: Vec<ResolveRule>,
    /// Optional arbitrator — overrides creator for dispute resolution.
    pub arbitrator: Option<Address>,
}

/// Parameters for a single invoice inside a `create_batch` call.
#[contracttype]
#[derive(Clone, Debug)]
pub struct CreateInvoiceParams {
    /// Recipient addresses.
    pub recipients: Vec<Address>,
    /// Amounts per recipient.
    pub amounts: Vec<i128>,
    /// Token addresses per recipient.
    pub tokens: Vec<Address>,
    /// Unix deadline timestamp.
    pub deadline: u64,
}

/// Optional notes field stored per invoice.
/// Set by the creator after invoice creation via `set_invoice_notes`.
/// Pure metadata — not evaluated in any payment or release logic.
#[contracttype]
#[derive(Clone, Debug)]
pub struct InvoiceNotes {
    /// Free-form text description or memo for this invoice.
    pub text: soroban_sdk::String,
    /// Ledger timestamp when notes were last updated.
    pub updated_at: u64,
}

/// Aggregated statistics for an invoice. Returned by `get_invoice_stats`.
#[contracttype]
#[derive(Clone, Debug)]
pub struct InvoiceStats {
    /// Total amount funded so far.
    pub funded: i128,
    /// Sum of all recipient amounts (the invoice's target).
    pub total: i128,
    /// Total number of individual payment transactions.
    pub payment_count: u32,
    /// Number of distinct payer addresses.
    pub unique_payers: u32,
    /// Funding completion in basis points: `funded * 10_000 / total`. 10_000 = fully funded.
    pub completion_bps: u32,
}

/// Memo metadata attached to an invoice for off-chain reference.
/// Stored and retrievable but not used in any payment or release logic.
#[contracttype]
#[derive(Clone, Debug)]
pub struct InvoiceMemo {
    /// Free-form text note visible on-chain.
    pub text: soroban_sdk::String,
    /// Ledger timestamp when the memo was created.
    pub created_at: u64,
}

/// Tags attached to an invoice for categorization and search.
/// Stored per-invoice via `set_invoice_tags` / `get_invoice_tags`.
/// Max 10 tags, each up to 32 chars — enforced at contract level.
#[contracttype]
#[derive(Clone, Debug)]
pub struct InvoiceTags {
    /// Ordered list of tags (e.g. ["freelance", "design", "urgent"]).
    pub tags: soroban_sdk::Vec<soroban_sdk::String>,
    /// Ledger timestamp when tags were last updated.
    pub updated_at: u64,
}

/// Extended memo field for invoices — separate from notes/tags.
/// Stores a short string memo updated by creator, with timestamp.
#[contracttype]
#[derive(Clone, Debug)]
pub struct InvoiceExtraMemo {
    /// Memo text (max 256 chars enforced at contract level).
    pub memo: soroban_sdk::String,
    /// Ledger timestamp of last update.
    pub updated_at: u64,
}

/// Generic metadata map for invoices — stored as Vec of key-value strings.
#[contracttype]
#[derive(Clone, Debug)]
pub struct InvoiceMetadata {
    /// Metadata entries as strings (e.g. "department:finance").
    pub entries: soroban_sdk::Vec<soroban_sdk::String>,
    /// Last update timestamp.
    pub updated_at: u64,
}

/// Discount configuration per invoice — basis points off total.
#[contracttype]
#[derive(Clone, Debug)]
pub struct DiscountConfig {
    /// Discount in basis points (0-10000, 1000=10%).
    pub discount_bps: u32,
    /// Timestamp set.
    pub updated_at: u64,
}
