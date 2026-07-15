.PHONY: build optimize test deploy-testnet deploy-mainnet

WASM=target/wasm32v1-none/release/sharpy.wasm
OPTIMIZED=target/wasm32v1-none/release/sharpy.optimized.wasm
TESTNET_ID=CAYTIFPD6RFWVHMK5SPPUUIWWAAANHKOJB6GOAJS5SR5MBKZMEY2UODZ

build:
	cargo build --release --target wasm32v1-none

optimize: build
	stellar contract optimize --wasm $(WASM)

test:
	cargo test

deploy-testnet: optimize
	stellar contract deploy \
		--wasm $(OPTIMIZED) \
		--source alice \
		--network testnet

init-testnet:
	stellar contract invoke \
		--id $(TESTNET_ID) \
		--source alice \
		--network testnet \
		-- initialize \
		--admin $(ADMIN) \
		--treasury $(TREASURY)

deploy-mainnet: optimize
	stellar contract deploy \
		--wasm $(OPTIMIZED) \
		--source deployer \
		--rpc-url https://mainnet.sorobanrpc.com \
		--network-passphrase "Public Global Stellar Network ; September 2015"
