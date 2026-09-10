# Changelog

All notable changes to the Sharpy smart contract are documented here.

## [Unreleased]
- feat(contract): `get_fee_bps()` rate view + preview-vs-invoice consistency proof (emitting vs silent share math, release untouched) — feat/fee-consistency (closes #195)
- feat(contract): whitelist enforced uniformly in `pay`/`pay_with_tip`/`pool_pay` + `is_whitelisted_payer(id, payer)` view; `require!` message consistency — feat/whitelist-consistency (closes #194)
- feat(contract): `get_tranche_remaining_bps(id)` view + cumulative-bps invariant (`released + remaining == 10000`) — feat/tranche-remaining (closes #193)
- feat(contract): `resolve_route_chain(id, max_depth)` bounded follower + `get_route_chain_len(id)` cycle-safe depth view — feat/route-chain-depth (closes #192)
- feat(contract): `get_stream_state(id)` + `preview_vested(id)` pure views; `withdraw_vested` repeat-withdraw idempotency fix (`total - vested`), cancel edge docs — feat/stream-vest-preview (closes #191)
- feat(events): invoice_updated (`inv_upd`) now emitted on `set_discount` and all whitelist mutations (`set/add/remove`), after the field-specific event; `is_invoice_expired(id)` read-only helper mirrors the `refund`/`refund_batch` expiry trigger — feat/event-taxonomy (closes #183)
- feat(contract): `get_creator_invoices_paged(creator, limit, offset)` + `get_payer_invoices_paged(payer, limit, offset)` with total bounds guards (limit 0 / offset past end yield empty pages); unpaginated index queries unchanged — feat/index-pagination (closes #184)
- feat(contract): `preview_fee_for_invoice(id)` silent fee estimate on the invoice total (no state change, no events; release math untouched) + `get_ttl_hint(id)` seconds-until-deadline observability view (0 when expired/terminal) — feat/fee-ttl-helpers (closes #185)
- feat(contract): audit hardening with zero behavior change — checked `checked_mul/div/add/sub` across fee math, pay/pool_pay/tip funding, streaming vesting, tranche and release accounting; CEI ordering notes on `_release`/`refund`; auth-coverage review documenting permissionless entries and unchanged stream/route auth — feat/audit-hardening (closes #181)
- test(contract): boundary coverage for the five 0.3.0 modules — streaming cliff edges, route overwrite/missing/single-hop, tranche 10000bps cap and creator auth, whitelist open-list/add-idempotent/remove-empty, fee preview zero/full/truncation bounds — test/new-module-edges (closes #182)

## [0.3.0] - 2026-09-04
- feat: streaming payments — `create_stream`/`withdraw_vested`/`cancel_stream`/`top_up_stream` cliff-gated linear vesting (`StreamingState`, 170 tests)
- feat: composable routing — `set_route`/`get_route`/`resolve_route` pass-through hop (`ComposableRoute`, 174 tests)
- feat: tranche release — `release_tranche`/`get_released_bps` partial release in bps (`TrancheState`, 178 tests)
- feat: whitelist gating — `set/get_whitelist` + add/remove enforced in `pay` (`WhitelistState`, 181 tests)
- feat: protocol fee — `set/get_protocol_fee` + `preview_fee` bps cut (`FeeConfig`, 184 tests)
- feat: approval flow — multi-approver workflow — feat/approval-flow
- feat: archival — archive and restore invoices — Adds archive_invoice and is_archived query, creator-only, 3 
- feat: invoice templates — reusable invoice configs — Adds InvoiceTemplate struct and create/get_template function
- feat: discount config — set/get_discount — feat/discount-config
- feat: invoice metadata — set/get_invoice_metadata — Adds InvoiceMetadata (key-value map) via set/get with creato
- `set_invoice_memo_ext`/`get_invoice_memo_ext` — InvoiceExtraMemo (256 chars) — feat/invoice-memo-ext (138 tests)
- feat: recurring pause — pause/resume recurring chain — Adds pause_recurring / resume_recurring and is_recurring_pau
- feat: deadline extension — extend_deadline for creators — Adds extend_deadline(caller, id, new_deadline) allowing crea
- feat: batch refund — refund_batch for multiple invoices — Adds refund_batch(caller, ids) to refund multiple deadline-p
 - 2026-08-28 — 30 PR day: 7 features + 12 test PRs + docs (120 tests passing)

### Added
- `get_invoice_version(id)` — returns invoice schema version (always 1) — `feat/get-invoice-version` + test PR #142
- `get_treasury()` — admin query returning initialized treasury address — `feat/get-treasury` + test PR #142
- `pay_with_tip(payer, id, amount, tip)` — gratuity routed to treasury, excluded from `funded` — `feat/pay-with-tip` + test PR #143
- `freeze_invoice(id)` / `unfreeze_invoice(id)` — admin freeze blocks `pay`/`pay_with_tip`, `frozen` field guard — `feat/freeze-unfreeze-invoice` + test PR #144 (4 tests)
- `get_recurring_params(id)` — exposes full `SubscriptionParams` (None for non-recurring) — `feat/get-recurring-params` + test PR #141
- `set_invoice_notes(caller, id, text)` / `get_invoice_notes(id)` — creator free-text `InvoiceNotes { text, updated_at }` — manual feat/invoice-notes + test PR #145 + extra 3 tests #146
- `get_invoices_by_creator` / `get_invoices_by_payer` / `get_invoice_count` already on main, now with new features integrated
- `refactor: require!` macro — consistent panic messages — PR #146 docs
- `refactor: _refund_payers` helper — deduplicates refund logic across `refund`, `cancel_invoice`, `resolve_dispute` — PR #146 docs
- Module-level `//!` doc on `lib.rs:1` covering 30+ capabilities (PR #146)
- 28 new tests across PRs #136-#146 (pool_pay already-released, interval=0, audit_log empty, release double, deadline past, recurring params, treasury/version, tip, freeze, notes extra, edge cases) — 92→120

### Changed
- `pay()` and `pay_with_tip()` now check `invoice.frozen` before processing (`"invoice is frozen"` panic)
- `types.rs` adds `InvoiceNotes` struct; `lib.rs` imports `String` and `InvoiceNotes`, adds `invoice_notes_key`
- `README.md` badge 92→120, Functions table + Project Structure updated
- Crate version `0.1.0` → `0.3.0` to match the release (`contracts/sharpy/Cargo.toml`); README badge and Deployments row aligned

### Verified
- `cargo test -p sharpy` — 184 passed on the 0.3.0 release line
- `cargo test` — 120 passed (was 92)
- `npm run build` sharpy-app — success with 10 PRs #157-#166 + SDK 11 PRs #77-#87

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
