# Sharpy — Advanced Split Payment Contract

![Soroban](https://img.shields.io/badge/Soroban-Protocol%2027-6C63FF?logo=stellar)
![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)
![Tests](https://img.shields.io/badge/tests-21%20passing-00D4AA)
![License](https://img.shields.io/badge/license-MIT-green)

Soroban smart contract powering the Sharpy split payment protocol on Stellar.

## Deployments

| Network | Contract ID | Status |
|---------|-------------|--------|
| Testnet | `CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ` | Live |
| Mainnet | Coming soon | Pending |

- [Testnet Explorer](https://stellar.expert/explorer/testnet/contract/CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ)
- [Frontend dApp](https://sharpy-sigma.vercel.app)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Sharpy Contract                          │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   Invoice    │  │   Payment    │  │     Escrow       │  │
│  │  Creation    │  │   Engine     │  │    Manager       │  │
│  │              │  │              │  │                  │  │
│  │ create       │  │ pay()        │  │ release_escrow() │  │
│  │ create_batch │  │ pool_pay()   │  │ dispute logic    │  │
│  │ create_recur │  │              │  │ arbitrator auth  │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
│         │                 │                   │             │
│         └─────────────────┴───────────────────┘             │
│                           │                                 │
│                  ┌────────▼────────┐                        │
│                  │  Invoice State  │                        │
│                  │                 │                        │
│                  │  Pending        │                        │
│                  │  Released       │                        │
│                  │  Refunded       │                        │
│                  │  Cancelled      │                        │
│                  └────────┬────────┘                        │
│                           │                                 │
│            ┌──────────────┴──────────────┐                  │
│            │                             │                  │
│   ┌────────▼────────┐        ┌───────────▼──────────┐       │
│   │   Split Rules   │        │    Audit Log         │       │
│   │                 │        │                      │       │
│   │ Fixed(amount)   │        │ pay / pool_pay       │       │
│   │ Percentage(bps) │        │ release / refund     │       │
│   │ Tiered(t, bps)  │        │ cancel               │       │
│   └─────────────────┘        └──────────────────────┘       │
└─────────────────────────────────────────────────────────────┘

                    Soroban Persistent Storage
┌─────────────────────────────────────────────────────────────┐
│  inv:{id}     Invoice struct (TTL ~1 year)                  │
│  log:{id}     AuditEntry vec                                │
│  rec:{id}     SubscriptionParams (recurring)                │
│  next_inv:{id} Next invoice ID in recurring chain           │
│  escrow:{id}  Escrow release timestamp                      │
│  counter      Global invoice ID counter                     │
│  admin        Admin address                                 │
│  treasury     Treasury address                              │
└─────────────────────────────────────────────────────────────┘
```

## Invoice Lifecycle

```
Creator calls create_invoice()
          │
          ▼
    ┌─────────────┐
    │   PENDING   │◄─────────────────────────────┐
    └──────┬──────┘                               │
           │                                      │
    pay() / pool_pay()                    (recurring: next
           │                               invoice created)
    funded >= total?                              │
           │                                      │
     ┌─────▼─────┐                               │
     │ escrow    │                               │
     │ enabled?  │                               │
     └─────┬─────┘                               │
           │                                     │
     ┌─────▼──────┐   delay passed   ┌───────────┴──────┐
     │  ESCROW    ├─────────────────►│   RELEASED       │
     │  LOCKED    │                  │ (funds split to  │
     └────────────┘                  │  recipients)     │
                                     └──────────────────┘
           │
    deadline passed,
    not fully funded
           │
           ▼
    ┌─────────────┐
    │  REFUNDED   │  (all payers refunded proportionally)
    └─────────────┘

    Creator calls cancel_invoice()
           │
           ▼
    ┌─────────────┐
    │  CANCELLED  │  (or REFUNDED if payments existed)
    └─────────────┘
```

## Features

- Multi-recipient invoice creation with configurable split rules
- **Split rules:** Fixed, Percentage, Tiered (threshold-based)
- Recurring/subscription invoices — auto-generates next on release
- Escrow protection with configurable release delay and arbitrator
- Batch invoice creation (up to 10 per transaction)
- Pool payments across multiple invoices
- Multi-token support — one token per recipient
- Full audit log per invoice
- Admin pause/unpause circuit breaker
- Storage TTL auto-extended (~1 year) on every write

## Protocol Compatibility

| Version | soroban-sdk | Status |
|---------|-------------|--------|
| Current | 26.1.0 | Protocol 27 ready |

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
│       ├── events.rs               # Event helpers
│       └── test.rs                 # 21 unit tests
└── .github/
    ├── workflows/ci.yml            # Test + build on every PR
    └── ISSUE_TEMPLATE/             # Bug report, feature request
```

## Build & Test

```bash
make test                    # cargo test (21 passing)
stellar contract build       # build + optimize WASM
make deploy-testnet          # deploy to testnet
```

## Related Repos

| Repo | Description |
|------|-------------|
| [sharpy-sdk](https://github.com/stellar-sharpy/sharpy-sdk) | TypeScript SDK |
| [sharpy-app](https://github.com/stellar-sharpy/sharpy-app) | Next.js frontend |

## License

MIT
