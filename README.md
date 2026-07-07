# Sharpy — Advanced Split Payment Contract

![Soroban](https://img.shields.io/badge/Soroban-Protocol%2027-6C63FF?logo=stellar)
![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)
![Tests](https://img.shields.io/badge/tests-15%20passing-00D4AA)
![License](https://img.shields.io/badge/license-MIT-green)
![Version](https://img.shields.io/badge/version-0.1.0-6C63FF)

Soroban smart contract powering the Sharpy split payment protocol on Stellar.

## Deployments

| Network | Contract ID | Status |
|---------|-------------|--------|
| Testnet | `CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ` | ✅ Live |
| Mainnet | Coming soon | ⏳ Pending |

- [Testnet Explorer](https://stellar.expert/explorer/testnet/contract/CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ)
- [Frontend dApp](https://sharpy-sigma.vercel.app)

## Features

- Multi-recipient invoice creation with configurable split rules
- **Split rules:** Fixed, Percentage (validated ≤ 100%), Tiered (threshold-based)
- **Multi-token support** — one token per recipient
- Recurring/subscription invoices — auto-generates next on release
- **Escrow protection** with configurable release delay and optional arbitrator
- **Escrow dispute mechanism** — arbitrator can intervene before release
- Batch invoice creation (up to 10 per transaction)
- Pool payments across multiple invoices (multi-token)
- **Structured events** for all lifecycle actions
- `get_invoice_stats` — funded/total/completion_bps/unique_payers
- Full audit log per invoice
- Admin pause/unpause circuit breaker
- Storage TTL auto-extended (~1 year) on every write

## Protocol Compatibility

| Version | soroban-sdk | Status |
|---------|-------------|--------|
| Current | 26.1.0 | ✅ Protocol 27 ready |

## Project Structure

```
sharpy-contracts/
├── Cargo.toml                      # Workspace (soroban-sdk 26.1.0)
├── Makefile                        # Build/test/deploy commands
├── contracts/sharpy/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                  # All contract logic
│       ├── types.rs                # Invoice, SplitRule, AuditEntry, etc.
│       ├── events.rs               # Structured event helpers
│       └── test.rs                 # 15 unit tests
└── .github/
    ├── workflows/ci.yml            # Test + build on every PR
    └── ISSUE_TEMPLATE/             # Bug report, feature request
```

## Build & Test

```bash
make test                    # cargo test (15 passing)
make build                   # build WASM
make optimize                # optimize WASM with stellar contract optimize
make deploy-testnet          # deploy to testnet
make deploy-mainnet          # deploy to mainnet
```

## Contract Functions

| Function | Description |
|----------|-------------|
| `initialize(admin, treasury)` | Set admin and treasury addresses |
| `create_invoice(...)` | Create invoice with split rules and escrow options |
| `create_batch(...)` | Create up to 10 invoices in one transaction |
| `create_recurring(...)` | Create recurring invoice with auto-generation |
| `pay(payer, invoice_id, amount)` | Pay toward an invoice |
| `pool_pay(payer, payments)` | Pay multiple invoices in one call |
| `release_escrow(invoice_id)` | Release after escrow delay passes |
| `release(invoice_id)` | Manual release for fully funded invoice |
| `refund(invoice_id)` | Refund payers after deadline |
| `cancel_invoice(caller, invoice_id)` | Creator cancels and refunds |
| `get_invoice(id)` | Read full invoice state |
| `get_invoice_stats(id)` | Get funded/total/completion_bps |
| `get_audit_log(id)` | Full audit trail |
| `get_payer_total(id, payer)` | Total paid by address |
| `get_next_recurring(id)` | Next invoice in recurring chain |
| `pause / unpause` | Admin circuit breaker |

## Split Rules

| Type | Behaviour |
|------|-----------|
| `Fixed(amount)` | Pay exact amount regardless of funded total |
| `Percentage(bps)` | Pay `funded * bps / 10_000` (validated ≤ 100%) |
| `Tiered(threshold, bps)` | Pay percentage only if `funded > threshold` |

## Related Repos

| Repo | Description |
|------|-------------|
| [sharpy-sdk](https://github.com/stellar-sharpy/sharpy-sdk) | TypeScript SDK |
| [sharpy-app](https://github.com/stellar-sharpy/sharpy-app) | Next.js frontend |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, standards, and commit conventions.

## License

MIT
