# Sharpy Architecture

This document explains internal storage layout, state machines, recurring flow, TTL strategy, and event taxonomy for contributors.

## Storage Key Reference

All persistent/instance keys are derived via `symbol_short!` (max 9 chars) + typed tuples.

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

TTL extension: every `save_invoice` and index/balance write calls `extend_ttl(100_000, 6_307_200)` — bump to ~1 year if TTL < 100k ledgers (~6 days, CAP-78).

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
- Invoked in: `save_invoice`, index updates, `credit_account`, `set_invoice_notes`, and explicit bump.

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

Future: `invoice_updated` and `invoice_expired` are defined for mutation/expiry paths (see events.rs).

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
