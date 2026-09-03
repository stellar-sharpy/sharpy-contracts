//! Structured event helpers for the Sharpy contract.
//!
//! Each function publishes a typed event to the Soroban event ledger.
//! Off-chain indexers (SDK, subgraph, etc.) subscribe to these events to maintain
//! an up-to-date view of invoice state without polling `get_invoice`.
//!
//! ## Event Topic Convention
//! All events use a single-element topic tuple containing a short symbol that uniquely
//! identifies the event type. Payloads are typed `#[contracttype]` structs for
//! deterministic XDR encoding and easy deserialization in the SDK.

use soroban_sdk::{contracttype, symbol_short, Address, Env, Vec};

/// Payload for the `created` event — emitted on every invoice creation
/// (including batch and recurring invoice generation).
#[contracttype]
#[derive(Clone)]
pub struct InvoiceCreatedEvent {
    pub id: u64,
    pub creator: Address,
}

/// Payload for the `payment` event — emitted on every successful payment,
/// both from `pay()` and `pool_pay()`.
#[contracttype]
#[derive(Clone)]
pub struct PaymentReceivedEvent {
    pub invoice_id: u64,
    pub payer: Address,
    pub amount: i128,
}

/// Payload for the `released` event — emitted when all funds are distributed to recipients.
#[contracttype]
#[derive(Clone)]
pub struct InvoiceReleasedEvent {
    pub id: u64,
    pub funded: i128,
    pub recipient_count: u32,
    pub creator: Address,
}

/// Payload for the `refunded` event — emitted when all payers are returned their funds
/// (deadline-based refund or dispute resolution to refund).
#[contracttype]
#[derive(Clone)]
pub struct InvoiceRefundedEvent {
    pub id: u64,
    pub funded: i128,
    pub recipient_count: u32,
    pub creator: Address,
}

/// Payload for the `pyr` (payer refunded) event — emitted once per payer during a refund.
/// Accompanies every `refunded` event; one per unique payer address.
#[contracttype]
#[derive(Clone)]
pub struct PayerRefundedEvent {
    pub invoice_id: u64,
    pub payer: Address,
    pub amount: i128,
}

/// Emits the `created` event. Topic: `("created",)`.
pub fn invoice_created(env: &Env, id: u64, creator: &Address) {
    env.events().publish((symbol_short!("created"),), InvoiceCreatedEvent { id, creator: creator.clone() });
}

/// Emits the `payment` event. Topic: `("payment",)`.
pub fn payment_received(env: &Env, invoice_id: u64, payer: &Address, amount: i128) {
    env.events().publish((symbol_short!("payment"),), PaymentReceivedEvent { invoice_id, payer: payer.clone(), amount });
}

/// Emits the `released` event. Topic: `("released",)`.
pub fn invoice_released(env: &Env, id: u64, funded: i128, recipient_count: u32, creator: &Address) {
    env.events().publish((symbol_short!("released"),), InvoiceReleasedEvent { id, funded, recipient_count, creator: creator.clone() });
}

/// Emits the `refunded` event. Topic: `("refunded",)`.
pub fn invoice_refunded(env: &Env, id: u64, funded: i128, recipient_count: u32, creator: &Address) {
    env.events().publish((symbol_short!("refunded"),), InvoiceRefundedEvent { id, funded, recipient_count, creator: creator.clone() });
}

/// Emits the `pyr` (payer refunded) event. Topic: `("pyr",)`.
/// Fired once per payer during refund — callers should aggregate by invoice_id.
pub fn payer_refunded(env: &Env, invoice_id: u64, payer: &Address, amount: i128) {
    env.events().publish((symbol_short!("pyr"),), PayerRefundedEvent { invoice_id, payer: payer.clone(), amount });
}

/// Payload for the `dispute` event — emitted when the invoice creator raises an escrow dispute.
#[contracttype]
#[derive(Clone)]
pub struct DisputeRaisedEvent {
    pub invoice_id: u64,
    pub creator: Address,
}

/// Payload for the `dsprslv` (dispute resolved) event — emitted when an arbitrator resolves.
/// `release: true` means funds were released; `false` means payers were refunded.
#[contracttype]
#[derive(Clone)]
pub struct DisputeResolvedEvent {
    pub invoice_id: u64,
    pub resolver: Address,
    pub release: bool,
}

/// Emits the `dispute` event. Topic: `("dispute",)`.
pub fn dispute_raised(env: &Env, invoice_id: u64, creator: &Address) {
    env.events().publish((symbol_short!("dispute"),), DisputeRaisedEvent { invoice_id, creator: creator.clone() });
}

/// Emits the `dsprslv` event. Topic: `("dsprslv",)`.
pub fn dispute_resolved(env: &Env, invoice_id: u64, resolver: &Address, release: bool) {
    env.events().publish((symbol_short!("dsprslv"),), DisputeResolvedEvent { invoice_id, resolver: resolver.clone(), release });
}

/// Payload for the `claimed` event — emitted when a recipient claims a fallback balance.
#[contracttype]
#[derive(Clone)]
pub struct AccountBalanceClaimedEvent {
    pub account: Address,
    pub token: Address,
    pub amount: i128,
}

/// Emits the `claimed` event. Topic: `("claimed",)`.
pub fn account_balance_claimed(env: &Env, account: &Address, token: &Address, amount: i128) {
    env.events().publish((symbol_short!("claimed"),), AccountBalanceClaimedEvent { account: account.clone(), token: token.clone(), amount });
}

/// Payload for the `cancel` event — emitted when a creator cancels their invoice.
/// `refunded_amount` is 0 if no payments had been made.
#[contracttype]
#[derive(Clone)]
pub struct InvoiceCancelledEvent {
    pub invoice_id: u64,
    pub creator: Address,
    pub refunded_amount: i128,
}

/// Emits the `cancel` event. Topic: `("cancel",)`.
pub fn invoice_cancelled(env: &Env, invoice_id: u64, creator: &Address, refunded_amount: i128) {
    env.events().publish(
        (symbol_short!("cancel"),),
        InvoiceCancelledEvent { invoice_id, creator: creator.clone(), refunded_amount },
    );
}

/// Payload for the `esc_fund` event — emitted when a fully-funded escrow invoice enters hold.
/// Listeners can use `release_at` to schedule a release trigger.
#[contracttype]
#[derive(Clone)]
pub struct EscrowFundedEvent {
    pub invoice_id: u64,
    pub release_at: u64,
    pub funded: i128,
}

/// Emits the `esc_fund` event. Topic: `("esc_fund",)`.
/// Fired in both `pay()` and `pool_pay()` when an escrow-enabled invoice reaches full funding.
pub fn escrow_funded(env: &Env, invoice_id: u64, release_at: u64, funded: i128) {
    env.events().publish(
        (symbol_short!("esc_fund"),),
        EscrowFundedEvent { invoice_id, release_at, funded },
    );
}

/// Payload for the `pymt_idx` (payment indexed) event — emitted when a payer is added to the payer index.
/// Allows indexers to subscribe to payer index changes without polling `get_invoices_by_payer`.
#[contracttype]
#[derive(Clone)]
pub struct PaymentIndexedEvent {
    pub payer: Address,
    pub invoice_id: u64,
}

/// Emits the `pymt_idx` event. Topic: `("pymt_idx",)`.
/// Fired when `index_invoice_for_payer` adds a new invoice_id for a payer (deduplicated).
pub fn payment_indexed(env: &Env, payer: &Address, invoice_id: u64) {
    env.events().publish(
        (symbol_short!("pymt_idx"),),
        PaymentIndexedEvent { payer: payer.clone(), invoice_id },
    );
}

/// Payload for the `inv_upd` (invoice updated) event — emitted when mutable invoice fields are changed.
/// Immutable fields: `creator`, `recipients`, `amounts`, `tokens`, `deadline` (commit at creation).
/// Mutable fields: `frozen`, `notes`, `escrow_release_delay` (future), `arbitrator` (future).
#[contracttype]
#[derive(Clone)]
pub struct InvoiceUpdatedEvent {
    pub invoice_id: u64,
    pub updater: Address,
    pub timestamp: u64,
}

/// Emits the `inv_upd` event. Topic: `("inv_upd",)`.
/// Fired on any state-mutating invoice update (freeze/unfreeze, notes, future mutators).
pub fn invoice_updated(env: &Env, invoice_id: u64, updater: &Address) {
    env.events().publish(
        (symbol_short!("inv_upd"),),
        InvoiceUpdatedEvent { invoice_id, updater: updater.clone(), timestamp: env.ledger().timestamp() },
    );
}

/// Payload for the `expired` (invoice expired) event — emitted when deadline passes and refund() is called.
/// Distinct from `refunded` to let indexers distinguish manual expiry from dispute refunds.
#[contracttype]
#[derive(Clone)]
pub struct InvoiceExpiredEvent {
    pub invoice_id: u64,
    pub deadline: u64,
    pub funded: i128,
}

/// Emits the `expired` event. Topic: `("expired",)`.
/// Fired in `refund()` when `timestamp > deadline` before transitioning to Refunded.
pub fn invoice_expired(env: &Env, invoice_id: u64, deadline: u64, funded: i128) {
    env.events().publish(
        (symbol_short!("expired"),),
        InvoiceExpiredEvent { invoice_id, deadline, funded },
    );
}

/// Payload for the `tags` event — emitted when invoice tags are updated.
#[contracttype]
#[derive(Clone)]
pub struct InvoiceTagsUpdatedEvent {
    pub invoice_id: u64,
    pub updater: Address,
    pub tag_count: u32,
}

/// Emits the `tags` event. Topic: `("tags",)`.
pub fn invoice_tags_updated(env: &Env, invoice_id: u64, updater: &Address, tag_count: u32) {
    env.events().publish(
        (symbol_short!("tags"),),
        InvoiceTagsUpdatedEvent { invoice_id, updater: updater.clone(), tag_count },
    );
}

/// Payload for the `memo` event — emitted when extra memo is updated.
#[contracttype]
#[derive(Clone)]
pub struct InvoiceMemoExtUpdatedEvent {
    pub invoice_id: u64,
    pub updater: Address,
}
/// Emits the `memo` event. Topic: `("memo",)`.
pub fn invoice_memo_ext_updated(env: &Env, invoice_id: u64, updater: &Address) {
    env.events().publish((symbol_short!("memo"),), InvoiceMemoExtUpdatedEvent { invoice_id, updater: updater.clone() });
}

/// Payload for deadline extension event.
#[contracttype]
#[derive(Clone)]
pub struct DeadlineExtendedEvent {
    pub invoice_id: u64,
    pub old_deadline: u64,
    pub new_deadline: u64,
}
/// Emits deadline extended event.
pub fn deadline_extended(env: &Env, invoice_id: u64, old_deadline: u64, new_deadline: u64) {
    env.events().publish((symbol_short!("ext_dead"),), DeadlineExtendedEvent { invoice_id, old_deadline, new_deadline });
}

/// Event for metadata update.
#[contracttype]
#[derive(Clone)]
pub struct InvoiceMetadataUpdatedEvent { pub invoice_id: u64, pub updater: Address }
pub fn invoice_metadata_updated(env: &Env, invoice_id: u64, updater: &Address) {
    env.events().publish((symbol_short!("imeta"),), InvoiceMetadataUpdatedEvent { invoice_id, updater: updater.clone() });
}

#[contracttype]
#[derive(Clone)]
pub struct DiscountUpdatedEvent { pub invoice_id: u64, pub discount_bps: u32 }
pub fn discount_updated(env: &Env, invoice_id: u64, discount_bps: u32) {
    env.events().publish((symbol_short!("disc"),), DiscountUpdatedEvent { invoice_id, discount_bps });
}

#[contracttype]
#[derive(Clone)]
pub struct RecurringPausedEvent { pub invoice_id: u64, pub paused: bool }
pub fn recurring_paused(env: &Env, invoice_id: u64, paused: bool) {
    env.events().publish((symbol_short!("rpause"),), RecurringPausedEvent { invoice_id, paused });
}

#[contracttype]
#[derive(Clone)]
pub struct TemplateCreatedEvent { pub template_id: u64, pub creator: Address }
pub fn template_created(env: &Env, template_id: u64, creator: &Address) {
    env.events().publish((symbol_short!("tmpl"),), TemplateCreatedEvent { template_id, creator: creator.clone() });
}

#[contracttype]
#[derive(Clone)]
pub struct InvoiceApprovedEvent { pub invoice_id: u64, pub approver: Address }
pub fn invoice_approved(env: &Env, invoice_id: u64, approver: &Address) {
    env.events().publish((symbol_short!("appr"),), InvoiceApprovedEvent { invoice_id, approver: approver.clone() });
}

#[contracttype]
#[derive(Clone)]
pub struct InvoiceArchivedEvent { pub invoice_id: u64, pub archiver: Address }
pub fn invoice_archived(env: &Env, invoice_id: u64, archiver: &Address) {
    env.events().publish((symbol_short!("arch"),), InvoiceArchivedEvent { invoice_id, archiver: archiver.clone() });
}

#[contracttype]
#[derive(Clone)]
pub struct StreamingStartedEvent { pub invoice_id: u64, pub recipient: Address, pub amount: i128, pub start_at: u64, pub end_at: u64, pub cliff_at: u64 }
pub fn streaming_started(env: &Env, invoice_id: u64, recipient: &Address, amount: i128, start_at: u64, end_at: u64, cliff_at: u64) {
    env.events().publish((symbol_short!("strm"),), StreamingStartedEvent { invoice_id, recipient: recipient.clone(), amount, start_at, end_at, cliff_at });
}

#[contracttype]
#[derive(Clone)]
pub struct StreamingWithdrawnEvent { pub invoice_id: u64, pub recipient: Address, pub amount: i128 }
pub fn streaming_withdrawn(env: &Env, invoice_id: u64, recipient: &Address, amount: i128) {
    env.events().publish((symbol_short!("wdr"),), StreamingWithdrawnEvent { invoice_id, recipient: recipient.clone(), amount });
}

#[contracttype]
#[derive(Clone)]
pub struct StreamingCancelledEvent { pub invoice_id: u64 }
pub fn streaming_cancelled(env: &Env, invoice_id: u64) {
    env.events().publish((symbol_short!("cncl"),), StreamingCancelledEvent { invoice_id });
}

#[contracttype]
#[derive(Clone)]
pub struct StreamingToppedUpEvent { pub invoice_id: u64, pub amount: i128 }
pub fn streaming_topped_up(env: &Env, invoice_id: u64, amount: i128) {
    env.events().publish((symbol_short!("tup"),), StreamingToppedUpEvent { invoice_id, amount });
}
