# sharpy-contracts

Soroban smart contract powering the Sharpy split payment protocol on Stellar. Handles invoice creation, multi-recipient fund distribution, escrow, recurring subscriptions, and batch operations.

## Table of Contents

- [Overview](#overview)
- [Deployments](#deployments)
- [Architecture](#architecture)
- [Contract Functions](#contract-functions)
- [Data Types](#data-types)
- [Split Rules](#split-rules)
- [Events](#events)
- [Development](#development)
- [Testing](#testing)
- [Deployment](#deployment)
- [Security](#security)
- [Contributing](#contributing)
- [License](#license)

## Overview

Sharpy is a payment-splitting protocol built on Stellar Soroban. It allows a creator to issue an invoice specifying multiple recipients and the amount each should receive. Payers fund the invoice on-chain, and once fully funded the contract automatically distributes funds to recipients according to the configured split rules.

Key capabilities:

- **Multi-recipient splits** — distribute funds to any number of recipients in a single invoice
- **Split rules** — Fixed amounts, Percentage-based, or Tiered (threshold-triggered) splits evaluated at release time
- **Escrow** — hold funds for a configurable delay after full payment before releasing to recipients
- **Recurring invoices** — automatically create the next invoice in a series upon release
- **Batch creation** — create up to 10 invoices in a single transaction
- **Pool payments** — pay toward multiple invoices in a single transaction
- **Audit log** — immutable on-chain record of every action taken on an invoice
- **Admin controls** — pause and unpause the contract

## Deployments

| Network | Contract ID |
|---------|-------------|
| Testnet | `CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ` |
| Mainnet | Coming soon |

- [Testnet Explorer](https://stellar.expert/explorer/testnet/contract/CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ)
- [Frontend dApp](https://sharpy-sigma.vercel.app)

## Architecture

```
contracts/sharpy/src/
├── lib.rs        Main contract logic and all public functions
├── types.rs      Data structures: Invoice, Payment, SplitRule, AuditEntry, etc.
├── events.rs     Event publishing helpers
└── test.rs       Unit tests
```

All invoice state is stored in Soroban persistent storage with a TTL of approximately one year (6,307,200 ledgers at 5 seconds per ledger). Storage TTL is extended on every write.

## Contract Functions

### Initialization

#### `initialize(env, admin, treasury)`

Must be called once after deployment. Sets the admin address and treasury address. Cannot be called again once initialized.

| Parameter | Type | Description |
|-----------|------|-------------|
| `admin` | `Address` | Administrator address with pause/unpause rights |
| `treasury` | `Address` | Treasury address for future fee collection |

### Admin

#### `pause(env)`

Pauses the contract. All state-modifying functions will reject calls while paused. Requires admin auth.

#### `unpause(env)`

Resumes normal operation. Requires admin auth.

### Invoice Creation

#### `create_invoice(env, creator, recipients, amounts, token, deadline, options) -> u64`

Creates a single invoice. Returns the invoice ID.

| Parameter | Type | Description |
|-----------|------|-------------|
| `creator` | `Address` | Invoice creator, must sign the transaction |
| `recipients` | `Vec<Address>` | Recipient addresses |
| `amounts` | `Vec<i128>` | Amount in token stroops for each recipient |
| `token` | `Address` | Token contract address (e.g. USDC) |
| `deadline` | `u64` | Unix timestamp deadline for payment |
| `options` | `InvoiceOptions` | Escrow and split rule configuration |

`InvoiceOptions` fields:

| Field | Type | Description |
|-------|------|-------------|
| `escrow_enabled` | `bool` | Hold funds before release |
| `escrow_release_delay` | `Option<u64>` | Seconds to hold after full payment |
| `split_rules` | `Vec<SplitRule>` | Per-recipient split rules (empty = proportional) |
| `auto_resolve_rules` | `Vec<ResolveRule>` | Reserved for future use |

#### `create_batch(env, creator, invoices) -> Vec<u64>`

Creates up to 10 invoices in one transaction. Returns a vector of invoice IDs.

| Parameter | Type | Description |
|-----------|------|-------------|
| `creator` | `Address` | Must sign the transaction |
| `invoices` | `Vec<CreateInvoiceParams>` | Array of invoice parameters (max 10) |

#### `create_recurring(env, creator, recipients, amounts, token, deadline, recurrence_interval, max_recurrences) -> u64`

Creates a recurring invoice. When released, the contract automatically creates the next invoice in the series.

| Parameter | Type | Description |
|-----------|------|-------------|
| `recurrence_interval` | `u64` | Seconds between invoices |
| `max_recurrences` | `u32` | Maximum invoices to generate (0 = infinite) |

### Payments

#### `pay(env, payer, invoice_id, amount)`

Pay toward an invoice. Transfers tokens from the payer to the contract. If the invoice reaches full funding and escrow is not enabled, funds are distributed immediately.

| Parameter | Type | Description |
|-----------|------|-------------|
| `payer` | `Address` | Must sign the transaction and hold sufficient token balance |
| `invoice_id` | `u64` | Target invoice |
| `amount` | `i128` | Amount in token stroops |

Constraints:
- Invoice must be in `Pending` status
- Ledger timestamp must be at or before the invoice deadline
- Amount must not exceed the remaining unfunded balance

#### `pool_pay(env, payer, payments)`

Pay toward multiple invoices in one transaction. All invoices must use the same token.

| Parameter | Type | Description |
|-----------|------|-------------|
| `payments` | `Vec<InvoicePayment>` | List of `{ invoice_id, amount }` pairs |

### Escrow

#### `release_escrow(env, invoice_id)`

Releases an escrow-held invoice once the delay period has passed. The invoice must be fully funded and escrow must be enabled.

#### `release(env, invoice_id)`

Manually triggers release for a fully funded non-escrow invoice. Useful if auto-release did not occur.

### Refunds and Cancellation

#### `refund(env, invoice_id)`

Refunds all payers proportionally to their contributions. Can only be called after the invoice deadline has passed and the invoice is still in `Pending` status (not fully funded).

#### `cancel_invoice(env, caller, invoice_id)`

Creator cancels the invoice. If any payments have been made they are refunded. If no payments have been made the invoice is marked `Cancelled`.

### Read-only

#### `get_invoice(env, invoice_id) -> Invoice`

Returns the full invoice state.

#### `get_audit_log(env, invoice_id) -> Vec<AuditEntry>`

Returns the complete audit trail for an invoice.

#### `get_payer_total(env, invoice_id, payer) -> i128`

Returns the total amount paid toward an invoice by a specific address.

#### `get_next_recurring(env, invoice_id) -> Option<u64>`

Returns the next invoice ID in a recurring chain, if one has been generated.

## Data Types

### Invoice

```rust
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
}
```

### InvoiceStatus

```rust
pub enum InvoiceStatus {
    Pending,
    Released,
    Refunded,
    Cancelled,
}
```

### Payment

```rust
pub struct Payment {
    pub payer: Address,
    pub amount: i128,
    pub tip: i128,
}
```

### AuditEntry

```rust
pub struct AuditEntry {
    pub action: Symbol,
    pub actor: Address,
    pub timestamp: u64,
}
```

Audit actions: `pay`, `pool_pay`, `release`, `refund`, `cancel`.

## Split Rules

Split rules are evaluated per recipient at release time. If `split_rules` is empty, funds are distributed proportionally to each recipient's `amount` relative to the invoice total.

### Fixed

```rust
SplitRule::Fixed(amount: i128)
```

The recipient receives exactly `amount` stroops regardless of how much was funded.

### Percentage

```rust
SplitRule::Percentage(bps: u32)
```

The recipient receives `funded * bps / 10_000`. For example, `bps = 5000` = 50%.

### Tiered

```rust
SplitRule::Tiered(threshold: i128, bps: u32)
```

The recipient receives `funded * bps / 10_000` only if `funded > threshold`. If the threshold is not met, the recipient receives 0.

## Events

| Topic | Data | Description |
|-------|------|-------------|
| `created` | `{ id, creator }` | Invoice created |
| `payment` | `{ invoice_id, payer, amount }` | Payment received |
| `released` | `{ id }` | Invoice released to recipients |
| `refunded` | `{ id }` | Invoice refunded |
| `pyr` | `{ invoice_id, payer, amount }` | Individual payer refunded |

## Development

### Prerequisites

- Rust stable toolchain
- `wasm32-unknown-unknown` target

```bash
rustup target add wasm32-unknown-unknown
```

- Stellar CLI

```bash
cargo install --locked stellar-cli --features opt
```

### Build

```bash
make build
# or
cargo build --release --target wasm32-unknown-unknown
```

### Optimize

```bash
make optimize
# or
stellar contract build --optimize
```

## Testing

```bash
make test
# or
cargo test
```

Tests are located in `contracts/sharpy/src/test.rs`. All tests use `env.mock_all_auths()` to bypass auth checks and test contract logic in isolation.

Current test coverage:

- `test_create_invoice` — invoice creation and initial state
- `test_batch_create` — batch invoice creation returns correct IDs
- `test_cancel_invoice` — creator can cancel an unfunded invoice

## Deployment

### Testnet

```bash
make deploy-testnet
```

Then initialize:

```bash
make init-testnet ADMIN=<your_address> TREASURY=<your_address>
```

### Mainnet

```bash
make deploy-mainnet
```

Ensure the deployer account is funded with at least 5 XLM to cover WASM upload and contract instantiation fees.

## Security

- Do not report security vulnerabilities in public GitHub issues
- Contact the maintainers directly
- The contract does not hold admin keys on-chain — admin is an externally owned account

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT
