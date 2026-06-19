# Contributing to Sharpy Contracts

Thank you for your interest in contributing to Sharpy! This repo contains the core Soroban smart contract powering the Sharpy split payment protocol on Stellar.

## Getting Started

### Prerequisites

- Rust (stable)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- Stellar CLI: `cargo install --locked stellar-cli --features opt`

### Setup

```bash
git clone https://github.com/stellar-sharpy/sharpy-contracts.git
cd sharpy-contracts
cargo test
```

## How to Contribute

### Reporting Bugs

Open an issue using the **Bug Report** template. Include:
- What you expected vs what happened
- Steps to reproduce
- Relevant contract method and parameters

### Suggesting Features

Open an issue using the **Feature Request** template. Describe the use case and why it adds value to the Stellar ecosystem.

### Submitting a Pull Request

1. Fork the repo and create a branch from `main`
2. Make your changes with clear, focused commits
3. Ensure `cargo test` passes
4. Ensure `cargo build --release --target wasm32-unknown-unknown` succeeds
5. Open a PR against `main` with a clear description

## Code Standards

- Follow existing patterns in `lib.rs`
- All public contract functions must have a doc comment
- New features require at least one unit test in `test.rs`
- No `unwrap()` in production paths — use `expect("descriptive message")`

## Commit Messages

Use conventional commits:
- `feat:` new contract functionality
- `fix:` bug fix
- `test:` test additions
- `docs:` documentation only
- `chore:` build/config changes
- `refactor:` code restructure without behaviour change

## Security

Do not open public issues for security vulnerabilities. Email the maintainers directly. See `SECURITY.md` for details.

## License

By contributing, you agree your contributions will be licensed under the MIT License.
