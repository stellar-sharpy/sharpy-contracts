# Changelog

All notable changes to the Sharpy smart contract are documented here.

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
