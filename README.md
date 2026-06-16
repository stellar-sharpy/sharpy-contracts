# Sharpy — Advanced Split Payment Contract

**Sharpy** is a Soroban smart contract for advanced payment splitting on Stellar.

## Deployments

| Network | Contract ID | Status |
|---------|-------------|--------|
| Testnet | `CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ` | ✅ Live |
| Mainnet | _coming soon_ | ⏳ Pending |

- [Testnet Explorer](https://stellar.expert/explorer/testnet/contract/CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ)
- [Frontend dApp](https://sharpy-sigma.vercel.app)

## Features

- **Recurring/Subscription Splits** — Auto-generate next invoice on release
- **Escrow with Dispute Period** — Hold funds for configurable delay before release
- **Batch Invoice Creation** — Create up to 10 invoices in a single transaction
- **Split Rules** — Fixed, Percentage, and Tiered (threshold-based) splits

## Project Structure

```
sharpy-contracts/
├── Cargo.toml                      # Workspace config
├── Makefile                        # Build/deploy commands
├── contracts/sharpy/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                  # Contract logic
│       ├── types.rs                # Data structures
│       ├── events.rs               # Events
│       └── test.rs                 # Unit tests (3/3 passing)
└── .github/workflows/ci.yml        # CI: test + build WASM on every PR
```

## Build & Test

```bash
make test          # cargo test (3/3 passing)
make build         # cargo build --release --target wasm32-unknown-unknown
make optimize      # stellar contract optimize
```

## Deploy

```bash
make deploy-testnet   # deploy + initialize on testnet
make deploy-mainnet   # deploy to mainnet (requires funded wallet)
```

## Split Rules

| Type | Behaviour |
|------|-----------|
| `Fixed(amount)` | Pay exact amount regardless of funded total |
| `Percentage(bps)` | Pay `funded * bps / 10_000` |
| `Tiered(threshold, bps)` | Pay percentage only if `funded > threshold` |

## Related Repos

| Repo | Description |
|------|-------------|
| [sharpy-sdk](https://github.com/stellar-sharpy/sharpy-sdk) | TypeScript SDK |
| [sharpy-app](https://github.com/stellar-sharpy/sharpy-app) | Next.js frontend |

## License

MIT
