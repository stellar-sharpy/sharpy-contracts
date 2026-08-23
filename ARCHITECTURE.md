# Sharpy Contract Architecture

Internal reference for contributors. Public API usage lives in [README.md](README.md).

## Storage key reference

All keys use `symbol_short!` (≤9 bytes). Globals and per-entity records:

| Key helper | Symbol / shape | Type (logical) | Purpose |
|------------|----------------|----------------|---------|
| `admin_key()` | `admin` | `Address` | Contract admin |
| `paused_key()` | `paused` | `bool` | Circuit breaker |
| `treasury_key()` | `treasury` | `Address` | Treasury / fee destination |
| `counter_key()` | `counter` | `u64` | Next invoice id allocator |
| `invoice_key(id)` | `(inv, id)` | `Invoice` | Invoice record |
| `audit_log_key(id)` | `(log, id)` | `Vec<AuditEntry>` | Per-invoice audit trail |
| `escrow_state_key(id)` | `(escrow, id)` | escrow/dispute state | Escrow hold + dispute metadata |
| `recurring_params_key(id)` | `(rec, id)` | `SubscriptionParams` | Recurring chain parameters |
| `next_invoice_key(id)` | `(next_inv, id)` | `u64` | Forward link in recurring chain |
| `creator_index_key(addr)` | `(by_ctr, Address)` | index list | Invoices created by address |
| `payer_index_key(addr)` | `(by_pyr, Address)` | index list | Invoices a payer touched |
| claimable balance | `(acc_bal, account, token)` | `i128` | Fallback balances after failed recipient transfers |

Persistent entries are extended on write (see TTL strategy).

## Invoice lifecycle

```text
                 create_invoice / create_batch / create_recurring
                                    |
                                    v
                               +---------+
                     pay*      | Pending | -- cancel (unfunded) --> Cancelled
                  -----------> |         |
                               +----+----+
                                    |
                    fully funded + non-escrow release path
                    or escrow release after hold/dispute
                                    |
                    +---------------+----------------+
                    v               v                v
               Released         Refunded         Cancelled
           (recipients paid)  (payers repaid)  (no pay / creator cancel)
```

`InvoiceStatus`: `Pending` → `Released` | `Refunded` | `Cancelled`.

\* `pay`, `pool_pay`, and related payment entry points contribute funding while status is `Pending`.

## Escrow / dispute state machine

When an invoice is created with escrow options and becomes fully funded:

```text
  Pending + payments
        |
        |  funding reaches total
        v
  Escrow hold (esc_fund event, release_at set)
        |
        +-- after release_at (or authorized release) --> Released
        |
        +-- creator raises dispute --> Disputed
        |                                  |
        |                    resolve(release=true)  --> Released
        |                    resolve(release=false) --> Refunded (per-payer pyr events)
        |
        +-- refund / cancel paths per API guards --> Refunded | Cancelled
```

Arbitrator/resolver auth is enforced on dispute resolution entry points. Escrow state storage is removed when the invoice leaves the hold/dispute path.

## Recurring chain flow

1. `create_recurring` stores `SubscriptionParams` under `(rec, id)` and creates the first invoice (`num_created = 1`).
2. On successful **release** of invoice `id`, if `max_recurrences == 0` (unlimited) or `num_created < max_recurrences`, the contract mints the next invoice, copies recipients/amounts/tokens, advances the deadline by `recurrence_interval`, and writes `(next_inv, id) -> next_id`.
3. Off-chain indexers follow `next_inv` links or subscribe to `created` events from the same creator.

## TTL strategy

- Invoice and related persistent keys are extended on mutation paths (pay, release, dispute, cancel, claim, etc.) so active invoices do not archive mid-lifecycle.
- Permissionless TTL bump helpers (where exposed) let anyone keep cold-but-still-relevant invoices alive for indexers.
- Prefer extending on write; pure getters should remain free of ledger side effects unless explicitly documented.

Exact threshold/extend values live in `contracts/sharpy/src/lib.rs` helpers — re-read them before changing rent behavior.

## Event taxonomy

All events use a **single-element topic** tuple: `(symbol_short!("…"),)`. Payloads are `#[contracttype]` structs in `events.rs`.

| Topic symbol | When | Payload highlights |
|--------------|------|--------------------|
| `created` | Invoice created (single, batch item, or recurring generation) | `id`, `creator` |
| `payment` | Payment applied via `pay` / `pool_pay` | `invoice_id`, `payer`, `amount` |
| `esc_fund` | Escrow invoice reaches full funding | `invoice_id`, `release_at`, `funded` |
| `released` | Funds distributed to recipients | `id`, `funded`, `recipient_count`, `creator` |
| `refunded` | Invoice-level refund completed | `id`, `funded`, `recipient_count`, `creator` |
| `pyr` | Per-payer refund slice | `invoice_id`, `payer`, `amount` |
| `dispute` | Creator raises escrow dispute | `invoice_id`, `creator` |
| `dsprslv` | Dispute resolved | `invoice_id`, `resolver`, `release` (bool) |
| `cancel` | Creator cancels | `invoice_id`, `creator`, `refunded_amount` |
| `claimed` | Fallback `claim` of `acc_bal` | `account`, `token`, `amount` |

Audit log entries (separate from events) append short action symbols such as `pay`, `pool_pay`, `dispute`, `resolve`, `release`, `refund`, `cancel` onto `(log, id)`.

## Module map

| Path | Role |
|------|------|
| `contracts/sharpy/src/lib.rs` | Entry points, storage helpers, state transitions |
| `contracts/sharpy/src/types.rs` | `InvoiceStatus`, `SplitRule`, `SubscriptionParams`, … |
| `contracts/sharpy/src/events.rs` | Typed event publishers |

When changing lifecycle behavior, update this file and the event table in the same PR.