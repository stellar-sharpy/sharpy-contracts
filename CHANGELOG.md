# Changelog

All notable changes to the Sharpy smart contract are documented here.

## [0.2.0] - 2026-07-18

### Added
- `get_invoice_fingerprint(id)` — SHA-256 tamper-evident content hash (Protocol 25 CAP-75)
- `bump_invoice_ttl(id)` — public TTL extension to prevent archival (Protocol 26 CAP-78)
- `dispute_release(invoice_id)` — escrow dispute mechanism
- `resolve_dispute(invoice_id, release)` — arbitrator resolves dispute
- Optional `arbitrator` field in `InvoiceOptions`
- `get_invoice_stats` — funded/total/completion_bps/unique_payers
- Multi-token support — one token per recipient

### Changed
- Split calculations use checked arithmetic throughout (Protocol 26 CAP-82)
- WASM build target updated to `wasm32v1-none` for Rust 1.84+
- soroban-sdk upgraded to 26.1.0 (Protocol 27 ready)
- Redeployed on testnet: `CBJ7WNBHCO5LKM7LW33D7HUT7WZI5OROVPC7IJL3A6NT6HMVJ4XUWPHJ`
- CI updated to use `wasm32v1-none` target

### Fixed
- Storage TTL extended on every `save_invoice` call (CAP-78)
- Percentage split rule validation (sum ≤ 100%)
- `cancel_invoice` audit log entry

## [0.1.0] - 2026-06-01

### Added
- `initialize` — set admin and treasury addresses
- `create_invoice` — single invoice with split rules and escrow options
- `create_batch` — create up to 10 invoices in one transaction
- `create_recurring` — recurring invoice that auto-generates next invoice on release
- `pay` — pay toward an invoice with token transfer
- `pool_pay` — pay multiple invoices in a single call
- `release_escrow` — release escrow-held invoice after delay
- `release` — manual release for fully funded invoices
- `refund` — refund all payers after deadline passes
- `cancel_invoice` — creator cancels and refunds all payments
- `get_invoice` — read invoice state
- `get_audit_log` — full audit trail per invoice
- `get_payer_total` — total paid by a specific address
- `get_next_recurring` — get the next invoice in a recurring chain
- `pause` / `unpause` — admin circuit breaker
- Split rules: `Fixed`, `Percentage`, `Tiered(threshold, bps)`
- Events: `created`, `payment`, `released`, `refunded`, `pyr`
- CI: GitHub Actions — test + WASM build on every PR
- Deployed to Stellar testnet: `CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ`

### Fixed
- `SplitRule::Tiered` converted from named fields to tuple variant for `#[contracttype]` compatibility
- `symbol_short!` length violations in event publishing
- `Address::random()` replaced with `Address::generate()` for soroban-sdk v22
- `testutils` feature isolated to `dev-dependencies` to allow wasm32 build
