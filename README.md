# Sharpy — Advanced Split Payment Contract

![Soroban](https://img.shields.io/badge/Soroban-Protocol%2027-6C63FF?logo=stellar)
![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)
![Tests](https://img.shields.io/badge/tests-153%20passing-00D4AA)
![License](https://img.shields.io/badge/license-MIT-green)
![Version](https://img.shields.io/badge/version-0.2.0-6C63FF)
[![Demo](https://img.shields.io/badge/Demo-Watch%20on%20Loom-00D4AA?logo=loom)](https://www.loom.com/share/09aa4a78e0c944dcab866a7036fde24d)

Soroban smart contract powering the Sharpy split payment protocol on Stellar. Handles invoice creation, multi-recipient fund distribution, escrow management, recurring billing, and agentic payment integration.

<img width="1047" height="649" alt="image" src="https://github.com/user-attachments/assets/cd447ba4-6134-41b2-983e-30cb529605e8" />


---

## Deployments

| Network | Contract ID | Status |
|---------|-------------|--------|
| Testnet | `CCMN5OYWBWVVRIB3IDE2CCODM3CMGSMYQ7EV2UVBJ23DVIH2CL6FJRXP` | ✅ Live (2026-08-11) |
| Mainnet | Coming soon | ⏳ Pending |

- [Testnet Explorer](https://stellar.expert/explorer/testnet/contract/CCMN5OYWBWVVRIB3IDE2CCODM3CMGSMYQ7EV2UVBJ23DVIH2CL6FJRXP)
- [Frontend dApp](https://sharpy-sigma.vercel.app)
- [Pitch Deck](https://gamma.app/docs/Split-Payments-on-Stellar-s0et8z1agtva59n)
- [Demo Video](https://www.loom.com/share/09aa4a78e0c944dcab866a7036fde24d)

### Live Transaction Examples

Recent testnet transactions demonstrating all features:

- [Create Invoice #3](https://stellar.expert/explorer/testnet/tx/ce46bcef570a4c05f6348081126135c9f24165c5e470a6b51b923f423156c5da) — Basic invoice creation with XLM
- [Create Batch Invoice](https://stellar.expert/explorer/testnet/tx/97cee323bb5443ddc8439f9d99f5a34e585f8cf74872a6138c5f1456adb5ab90) — Multiple invoices in one call
- [Multi-recipient Payment](https://stellar.expert/explorer/testnet/tx/785d079c53350fdf50db1e6d92da2219e148b204b87b6448632d1e21a94faac4) — Split payment to multiple addresses
- [Escrow Invoice](https://stellar.expert/explorer/testnet/tx/db19f9206a4a25b4431b6a3dfae25080f3c20a285249521aac5e593f1c26e76c) — Invoice with escrow protection
- [Recurring Invoice](https://stellar.expert/explorer/testnet/tx/2f5e2344337de8f4c578f5d91861db4425ebcfcf967b4d1430c0434d9e77ea64) — Subscription invoice creation

**Test Account**: `GD4Q2BH6KISIHTZWV5CSUMZC7VUBQAAXPNVSCESTUGH5WEYALMOTRS63` ([View on Explorer](https://stellar.expert/explorer/testnet/account/GD4Q2BH6KISIHTZWV5CSUMZC7VUBQAAXPNVSCESTUGH5WEYALMOTRS63))

---

## Internal architecture

Contributor-oriented storage keys, state machines, and events: [ARCHITECTURE.md](ARCHITECTURE.md).

## Architecture

```mermaid
graph TD
    App["sharpy-app\nNext.js 14"]
    SDK["@stellar-sharpy/sdk"]
    RPC["Soroban RPC"]
    Contract["Sharpy Contract\nSoroban · Protocol 27"]
    Stellar["Stellar Network"]

    App -->|"calls"| SDK
    SDK -->|"simulate + submit"| RPC
    RPC -->|"executes"| Contract
    Contract -->|"ledger state + events"| Stellar
    Stellar -->|"events"| SDK
```

---

## Features

- **Multi-recipient invoices** — split funds to any number of recipients in one transaction
- **Split rules** — Fixed, Percentage (validated ≤ 100%), Tiered (threshold-based)
- **Multi-token support** — one token per recipient (USDC, XLM, AQUA, yXLM)
- **Recurring/subscription invoices** — auto-generates next invoice on release
- **Escrow protection** — configurable release delay with optional arbitrator
- **Escrow dispute mechanism** — arbitrator can intervene before release
- **Batch invoice creation** — up to 10 invoices in a single transaction
- **Pool payments** — pay multiple invoices across different tokens in one call
- **Structured events** — for all lifecycle actions (created, payment, released, refunded, cancelled, escrow_funded)
- **Invoice stats** — funded/total/completion_bps/unique_payers via `get_invoice_stats`
- **Full audit log** — on-chain audit trail per invoice
- **Admin circuit breaker** — pause/unpause contract
- **Payer index** — `get_invoices_by_payer` tracks all invoices a payer touched (via `pay` and `pool_pay`)
- **Creator index** — `get_invoices_by_creator` for dashboard pagination
- **Fallback balance recovery** — `claim()` + `get_claimable_balance()` for failed recipient transfers (CEI pattern)
- **Storage TTL auto-extended** — ~1 year on every write
- **Treasury & tips** — `get_treasury()` + `pay_with_tip()` routes gratuity to treasury, excluded from `funded`
- **Freeze control** — `freeze_invoice()`/`unfreeze_invoice()` admin blocks/re-enables `pay` (frozen field)
- **Invoice notes** — `set_invoice_notes()`/`get_invoice_notes()` free-text `InvoiceNotes { text, updated_at }`
- **Invoice tags**
- **Discount config** — `set/get_discount` `DiscountConfig { discount_bps, updated_at }`
- **Invoice metadata** — `set/get_invoice_metadata` `InvoiceMetadata { entries, updated_at }`
- **Deadline extension** — `extend_deadline(caller, id, new_deadline)` creator can push deadline forward
- **Batch refund** — `refund_batch(caller, ids)` refund up to 10 deadline-passed invoices in one tx
- **Extra memo** — `set_invoice_memo_ext()`/`get_invoice_memo_ext()` creator memo `InvoiceExtraMemo { memo, updated_at }` (256 chars) — `set_invoice_tags()`/`get_invoice_tags()` categorized `InvoiceTags { tags, updated_at }` (max 10, 32 chars each)
- **Recurring query** — `get_recurring_params()` exposes full `SubscriptionParams`
- **Version query** — `get_invoice_version()` returns schema version (1)

---

## Protocol 25/26 Features

| CAP | Protocol | Feature | Implementation |
|-----|----------|---------|---------------|
| CAP-82 | 26 | Checked 256-bit arithmetic | Overflow-safe split calculations in `_release()` and `get_invoice_stats()` |
| CAP-78 | 26 | Limited TTL extension host functions | `bump_invoice_ttl()` — anyone can extend invoice storage lifetime |
| CAP-75 | 25 | Poseidon/crypto host functions | `get_invoice_fingerprint()` — SHA-256 tamper-evident content hash |

---

## Contract Functions

| Function | Description |
|----------|-------------|
| `initialize(admin, treasury)` | Set admin and treasury addresses |
| `create_invoice(creator, recipients, amounts, tokens, deadline, options)` | Create invoice with split rules and escrow options |
| `create_batch(creator, invoices)` | Create up to 10 invoices in one transaction |
| `create_recurring(creator, recipients, amounts, token, deadline, interval, max)` | Create recurring invoice with auto-generation on release |
| `pay(payer, invoice_id, amount)` | Pay toward an invoice |
| `pool_pay(payer, payments)` | Pay multiple invoices in one call (multi-token) |
| `release_escrow(invoice_id)` | Release escrow-held funds after delay passes |
| `release(invoice_id)` | Manual release for fully funded invoice |
| `refund(invoice_id)` | Refund all payers after deadline passes |
| `cancel_invoice(caller, invoice_id)` | Creator cancels invoice and refunds payments |
| `dispute_release(invoice_id)` | Raise an escrow dispute before release |
| `resolve_dispute(invoice_id, release)` | Arbitrator resolves dispute — release or refund |
| `get_invoice(id)` | Read full invoice state |
| `get_invoice_stats(id)` | Get funded/total/completion_bps/payment_count/unique_payers |
| `get_invoice_fingerprint(id)` | SHA-256 tamper-evident content hash (Protocol 25/26) |
| `get_audit_log(id)` | Full audit trail as Vec<AuditEntry> |
| `get_payer_total(id, payer)` | Total amount paid by a specific address |
| `get_next_recurring(id)` | Next invoice ID in a recurring chain |
| `get_escrow_state(id)` | Current escrow/dispute state |
| `bump_invoice_ttl(id)` | Extend invoice storage TTL to prevent archival (Protocol 26 CAP-78) |
| `get_invoice_count()` | Total number of invoices ever created — O(1) global stat |
| `get_invoices_by_creator(creator)` | All invoice IDs created by an address |
| `get_invoices_by_payer(payer)` | All invoice IDs paid by a given address (payer index) |
| `get_claimable_balance(account, token)` | Claimable balance for account after failed transfer |
| `claim(account, token)` | Withdraw credited balance for account/token |
| `get_invoice_version(id)` | Invoice schema version (always 1) |
| `get_treasury()` | Treasury address set at `initialize` |
| `pay_with_tip(payer, id, amount, tip)` | Pay with gratuity routed to treasury (tip excluded from `funded`) |
| `freeze_invoice(id)` / `unfreeze_invoice(id)` | Admin freeze/unfreeze — blocks `pay`/`pay_with_tip` when frozen |
| `get_recurring_params(id)` | Full `SubscriptionParams` for recurring invoices (None if not recurring) |
| `set_invoice_notes(caller, id, text)` / `get_invoice_notes(id)` | Creator free-text notes `InvoiceNotes { text, updated_at }` |
| `set_invoice_tags(caller, id, tags)` / `get_invoice_tags(id)` | Creator tags `InvoiceTags { tags, updated_at }` (10 max) |
| `pause` / `unpause` | Admin circuit breaker |

---

## Split Rules

| Type | Behaviour | Example |
|------|-----------|---------|
| `Fixed(amount)` | Pay exact amount regardless of funded total | `Fixed(500_000_000)` → always 50 USDC |
| `Percentage(bps)` | Pay `funded * bps / 10_000` (validated ≤ 100%) | `Percentage(6000)` → 60% of funded |
| `Tiered(threshold, bps)` | Pay percentage only if `funded > threshold`, else 0 | `Tiered(100_000_000, 5000)` → 50% if funded > 10 USDC |

---

## Project Structure

```
sharpy-contracts/
├── Cargo.toml                       # Workspace (soroban-sdk 26.1.0)
├── Makefile                         # Build/test/deploy commands
├── CONTRIBUTING.md
├── SECURITY.md
├── CODE_OF_CONDUCT.md
├── CHANGELOG.md
├── contracts/sharpy/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                   # All contract logic (600+ lines)
│       ├── types.rs                 # Invoice, SplitRule, AuditEntry, etc.
│       ├── events.rs                # Structured event helpers
│       └── test.rs                  # 129 unit tests (+7 features + 12 test PRs 2026-08-28)
└── .github/
    ├── workflows/ci.yml             # Test + WASM build on every PR
    └── ISSUE_TEMPLATE/              # Bug report, feature request
```

---

## Build & Test

```bash
make test           # cargo test (129 passing)
make build          # build WASM
make optimize       # optimize WASM with stellar contract optimize
make deploy-testnet # deploy to testnet
make deploy-mainnet # deploy to mainnet
```

---

## Protocol Compatibility

| soroban-sdk | Protocol | Status |
|-------------|----------|--------|
| 26.1.0 | 27 | ✅ Current |

---

## Related Repos

| Repo | Description |
|------|-------------|
| [sharpy-sdk](https://github.com/stellar-sharpy/sharpy-sdk) | TypeScript SDK |
| [sharpy-app](https://github.com/stellar-sharpy/sharpy-app) | Next.js frontend dApp |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, standards, and commit conventions.

## Security

See [SECURITY.md](SECURITY.md) for the vulnerability disclosure process.

## License

[MIT](LICENSE)
