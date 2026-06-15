# Sharpy — Advanced Split Payment Contract

**Sharpy** is a Soroban smart contract designed for advanced payment splitting on Stellar, featuring:

## Key Features

### ✅ MVP (Phase 1)
1. **Recurring/Subscription Splits** — Automatically generate the next invoice upon release
2. **Escrow with Dispute Period** — Hold funds for configurable delay before release
3. **Batch Invoice Creation** — Create up to 10 invoices in a single transaction
4. **Payment Scheduling** — Split rules: Fixed, Percentage, Tiered (dynamic at release)

### 🚀 Roadmap (Phase 2+)
- Graduated release (tranches with time-based unlocking)
- Cross-chain references
- Advanced oracle integration
- Multi-token support with DEX swaps
- Receipt tokens for payment proofs
- Creator whitelisting
- Compliance checks

## Deployments

| Network | Contract ID |
|---------|-------------|
| Testnet | `CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ` |
| Mainnet | _coming soon_ |

- [Testnet Explorer](https://stellar.expert/explorer/testnet/contract/CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ)

## Project Structure

```
sharpy-contracts/
├── Cargo.toml                      # Workspace config
├── contracts/sharpy/
│   ├── Cargo.toml                  # Contract package
│   └── src/
│       ├── lib.rs                  # Main contract logic
│       ├── types.rs                # Data structures
│       ├── events.rs               # Event definitions
│       └── test.rs                 # Unit tests
└── README.md
```

## Building

```bash
cargo build --release --target wasm32-unknown-unknown
stellar contract optimize --wasm target/wasm32-unknown-unknown/release/sharpy.wasm
```

## Testing

```bash
cargo test
```

## Contract Initialization

```rust
let contract = SharpyContractClient::new(&env, &contract_address);
contract.initialize(&admin, &treasury);
```

## Key Operations

### Create Invoice
```rust
let invoice_id = contract.create_invoice(
    &creator,
    &recipients,
    &amounts,
    &token,
    &deadline,
    &options,  // InvoiceOptions { escrow_enabled, escrow_release_delay, split_rules, .. }
);
```

### Batch Create (up to 10 invoices)
```rust
let ids = contract.create_batch(&creator, &invoice_params);
```

### Create Recurring Invoice
```rust
let invoice_id = contract.create_recurring(
    &creator,
    &recipients,
    &amounts,
    &token,
    &deadline,
    &recurrence_interval,  // seconds
    &max_recurrences,      // 0 = infinite
);
```

### Pay
```rust
contract.pay(&payer, &invoice_id, &amount);
```

### Batch Pay (multiple invoices)
```rust
contract.pool_pay(&payer, &payments);
```

### Release (if escrow)
```rust
contract.release_escrow(&invoice_id);
```

## Escrow Flow

1. **Payment received** → funds locked in contract
2. **Escrow delay passes** → caller invokes `release_escrow()`
3. **Funds distributed** → to recipients according to split rules

## Split Rules

### Fixed Amount
Pay recipient exactly this amount regardless of total funded.

### Percentage
Pay recipient `funded * bps / 10_000` (in basis points).

### Tiered
Pay recipient percentage **only** if `funded > threshold`, else 0.
Encoded as `SplitRule::Tiered(threshold, bps)`.

## License

MIT
