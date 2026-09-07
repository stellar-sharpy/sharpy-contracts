# Sharpy Architecture

This document explains internal storage layout, state machines, recurring flow, TTL strategy, and event taxonomy for contributors.

## Storage Key Reference

All keys are derived via `symbol_short!` (max 9 chars). Singleton config (`admin`, `treasury`, `fee`) lives in
instance storage (no TTL); every per-invoice entry plus the counters lives in persistent storage as typed tuples.

| Key Symbol | Type | Storage | Description |
|------------|------|---------|-------------|
| `admin` | `Address` | instance | Admin set in `initialize` |
| `treasury` | `Address` | instance | Treasury for tips |
| `paused` | `bool` | persistent | Circuit breaker |
| `counter` | `u64` | persistent | Global invoice ID counter |
| `("inv", id)` | `Invoice` | persistent | Invoice body |
| `("log", id)` | `Vec<AuditEntry>` | persistent | Per-invoice audit log |
| `("escrow", id)` | `DisputeState` | persistent | Escrow hold state |
| `("rec", id)` | `SubscriptionParams` | persistent | Recurring params |
| `("next_inv", id)` | `u64` | persistent | Next recurring invoice pointer |
| `("by_ctr", creator)` | `Vec<u64>` | persistent | Creator index |
| `("by_pyr", payer)` | `Vec<u64>` | persistent | Payer index |
| `("acc_bal", account, token)` | `i128` | persistent | Claimable fallback balance |
| `("notes", id)` | `InvoiceNotes` | persistent | Free-text notes |
| `("itags", id)` | `InvoiceTags` | persistent | Categorized tags via `set/get_invoice_tags` |
| `("arch", id)` | `ArchivalState` | persistent | Terminal archive flag via `archive/unarchive/is_archived` |
| `("appr", id)` | `ApprovalState` | persistent | Multi-approver config via `set_approval_config`/`approve_invoice` |
| `("tmpl", template_id)` | `InvoiceTemplate` | persistent | Reusable configs via `create_template`/`get_template` |
| `tmpl_ctr` | `u64` | persistent | Global template ID counter |
| `("rpause", id)` | `RecurringPauseState` | persistent | Recurring-chain pause via `pause/resume_recurring` |
| `("disc", id)` | `DiscountConfig` | persistent | Discount bps via `set/get_discount` |
| `("imeta", id)` | `InvoiceMetadata` | persistent | Key-value entries via `set/get_invoice_metadata` |
| `("imemo", id)` | `InvoiceExtraMemo` | persistent | 256-char memo via `set/get_invoice_memo_ext` |
| `("strm", id)` | `StreamingState` | persistent | Cliff-gated vesting via `create_stream`/`withdraw_vested`/`cancel_stream`/`top_up_stream` |
| `("route", id)` | `ComposableRoute` | persistent | Pass-through hop via `set/get/resolve_route` |
| `("tranche", id)` | `TrancheState` | persistent | Partial-release accounting via `release_tranche`/`get_released_bps` |
| `("wlist", id)` | `WhitelistState` | persistent | Payer allowlist enforced in `pay` via `set/get/add/remove_whitelisted_payer` |
| `fee` | `FeeConfig` | instance | Protocol fee bps + collector via `set/get_protocol_fee`/`preview_fee` |

TTL extension: `save_invoice`, creator/payer index writes, `credit_account`, `bump_invoice_ttl`, `set_invoice_notes`,
`set_invoice_tags`, `set_invoice_memo_ext`, `set_invoice_metadata` and `set_discount` call
`extend_ttl(100_000, 6_307_200)` — bump to ~1 year if TTL < 100k ledgers (~6 days, CAP-78).
Instance singletons (`admin`, `treasury`, `fee`) carry no TTL; escrow, recurring, pause, approval, archival,
streaming, route, tranche, whitelist and template writes ride on the invoice/index entries above.

## Invoice Lifecycle State Machine

```
          create_invoice / create_batch / create_recurring
                        |
                        v
                     Pending
                        |
        +---------------+---------------+----------------+
        |               |               |                |
     pay (>=total)   refund(*)    cancel_invoice    freeze/unfreeze
        |            (deadline>)       |               (admin, stays Pending)
        v               v              v
     Released        Refunded      Cancelled/Refunded
        |                             (funded?Refund:Cancel)
        v
   next recurring? -> spawn Pending (chain)
```

* `refund` callable when `timestamp > deadline` on Pending invoices.
* `_release` is internal; it sets `Released` and `completion_time`, emits `released`.
* `cancel_invoice` by creator -> Refunded if funded>0 else Cancelled.
* `freeze_invoice` blocks `pay()`; `unfreeze` restores.

### Escrow / Dispute State Machine

```
pay fully funded && escrow_enabled -> DisputeState{release_at, disputed=false} + esc_fund event
        |
   +----+----+
   |         |
dispute_release  release_escrow (after release_at && !disputed)
   |         |
   v         v
Disputed   Released
   |
resolve_dispute(release=true/false) -> Released / Refunded + dsprslv/dispute events
```

Guards: `dispute_release` requires `timestamp < release_at` and creator auth; `release_escrow` panics if disputed; `resolve_dispute` uses arbitrator if set else creator.

## Recurring Chain Flow

1. `create_recurring(creator, ..., interval, max_recurrences)` -> id= N, stores `SubscriptionParams{creator, recipients, amounts, tokens, interval, max, num_created=1}`.
2. On `_release` (via pay), if `params` exists and `max==0 || num_created < max`, spawn next invoice: `deadline = now + interval`, id= N+1, `num_created+1`, set `next_inv[N]=N+1`, emit `created`.
3. Each spawned invoice carries its own copy of `SubscriptionParams` with incremented `num_created`, so chain continues independently.
4. `max_recurrences=1` means only the genesis invoice; no next.

## TTL Strategy (CAP-78)

- Soroban persistent entries expire. Sharpy extends TTL on every write using `extend_ttl(min=100k, max=6.3M)`.
- `bump_invoice_ttl(id)` is a manual keep-alive for long-lived invoices.
- Invoked in: `save_invoice`, creator/payer index updates, `credit_account`, `set_invoice_notes`, `set_invoice_tags`, `set_invoice_memo_ext`, `set_invoice_metadata`, `set_discount`, and explicit bump.

## Event Taxonomy

All events use single-element topic `symbol_short!`.

| Topic | Struct | Emitted by |
|-------|--------|------------|
| `created` | `InvoiceCreatedEvent{id, creator}` | `create_invoice`, `create_batch`, `create_recurring`, recurring spawn |
| `payment` | `PaymentReceivedEvent{invoice_id, payer, amount}` | `pay`, `pool_pay`, `pay_with_tip` |
| `pymt_idx` | `PaymentIndexedEvent{payer, invoice_id}` | `index_invoice_for_payer` (deduplicated) |
| `released` | `InvoiceReleasedEvent{id, funded, recipient_count, creator}` | `_release` |
| `refunded` | `InvoiceRefundedEvent{id, funded, recipient_count, creator}` | `refund`, `resolve_dispute(refund)` |
| `pyr` | `PayerRefundedEvent{invoice_id, payer, amount}` | `_refund_payers` (per unique payer) |
| `dispute` | `DisputeRaisedEvent{invoice_id, creator}` | `dispute_release` |
| `dsprslv` | `DisputeResolvedEvent{invoice_id, resolver, release}` | `resolve_dispute` |
| `claimed` | `AccountBalanceClaimedEvent{account, token, amount}` | `claim` |
| `cancel` | `InvoiceCancelledEvent{invoice_id, creator, refunded_amount}` | `cancel_invoice` |
| `esc_fund` | `EscrowFundedEvent{invoice_id, release_at, funded}` | `pay`/`pool_pay` full funding with escrow |
| `inv_upd` | `InvoiceUpdatedEvent{invoice_id, updater, timestamp}` | `freeze`/`unfreeze`, `set_invoice_notes`, `set_invoice_tags`, `set_invoice_memo_ext`, `extend_deadline`, `set_invoice_metadata` |
| `expired` | `InvoiceExpiredEvent{invoice_id, deadline, funded}` | `refund`, `refund_batch` (per deadline-passed invoice) |
| `tags` | `InvoiceTagsUpdatedEvent{invoice_id, updater, tag_count}` | `set_invoice_tags` |
| `memo` | `InvoiceMemoExtUpdatedEvent{invoice_id, updater}` | `set_invoice_memo_ext` |
| `ext_dead` | `DeadlineExtendedEvent{invoice_id, old_deadline, new_deadline}` | `extend_deadline` |
| `imeta` | `InvoiceMetadataUpdatedEvent{invoice_id, updater}` | `set_invoice_metadata` |
| `disc` | `DiscountUpdatedEvent{invoice_id, discount_bps}` | `set_discount` |
| `rpause` | `RecurringPausedEvent{invoice_id, paused}` | `pause_recurring`/`resume_recurring` |
| `tmpl` | `TemplateCreatedEvent{template_id, creator}` | `create_template` |
| `appr` | `InvoiceApprovedEvent{invoice_id, approver}` | `set_approval_config`/`approve_invoice` |
| `arch` | `InvoiceArchivedEvent{invoice_id, archiver}` | `archive_invoice` |
| `strm` | `StreamingStartedEvent{invoice_id, recipient, amount, start_at, end_at, cliff_at}` | `create_stream` |
| `wdr` | `StreamingWithdrawnEvent{invoice_id, recipient, amount}` | `withdraw_vested` |
| `cncl` | `StreamingCancelledEvent{invoice_id}` | `cancel_stream` |
| `tup` | `StreamingToppedUpEvent{invoice_id, amount}` | `top_up_stream` |
| `route` | `RouteSetEvent{invoice_id, target_invoice}` | `set_route` |
| `rslv` | `RouteResolvedEvent{invoice_id, target_invoice}` | `resolve_route` |
| `tranch` | `TrancheReleasedEvent{invoice_id, bps, cumulative_bps}` | `release_tranche` |
| `wlist` | `WhitelistSetEvent{invoice_id, payer_count}` | `set_whitelist`/`add_whitelisted_payer` |
| `wrem` | `WhitelistPayerRemovedEvent{invoice_id, payer}` | `remove_whitelisted_payer` |
| `fee` | `FeeConfiguredEvent{fee_bps, collector}` | `set_protocol_fee` |
| `fprev` | `FeePreviewedEvent{amount, fee}` | `preview_fee` |

## Checked Arithmetic (CAP-82)

All payout math uses `checked_mul`/`checked_div`/`checked_add`/`checked_sub` to prevent overflow — important for `Percentage`/`Tiered` splits computing `funded * bps / 10000` on `i128`.

## Security Notes

- `claim()` follows CEI: `remove` before `transfer` (Soroban is non-reentrant but defense-in-depth).
- `pay()` is sequential per ledger; concurrent txs are applied serially, so `total - funded` guard cannot be bypassed; overpayment attempts panic with explicit remaining amount.
- `credit_account` uses `checked_add`.

## Module Map

- `contracts/sharpy/src/lib.rs` — contract impl, storage helpers, `SharpyContract`
- `contracts/sharpy/src/events.rs` — typed event helpers
- `contracts/sharpy/src/types.rs` — `Invoice`, `SplitRule`, `DisputeState`, etc.
- `contracts/sharpy/src/test.rs` — 120+ unit/integration tests
