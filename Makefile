.PHONY: init
init:
	./init.sh

.PHONY: check
check:
	SKIP_WASM_BUILD=1 cargo check --release

.PHONY: test
test:
	SKIP_WASM_BUILD=1 cargo test --release --all

.PHONY: run
run:
	 cargo run --release -- --dev --tmp

.PHONY: build
build:
	 cargo build --release

.PHONY: run target
run target:
	./target/release/bandot-node --dev

./PHONY: purge
purge:
	./target/release/bandot-node purge-chain --dev

./PHONY: rpc
rpc:
	./target/release/bandot-node  --dev --ws-external --unsafe-rpc-external --rpc-cors=all