# Development tasks.
#
# CI runs these same targets, so what passes here is what passes there — the gate list
# lives in one place rather than being duplicated into the workflow.

.PHONY: all ci fmt fmt-check lint test build run demo clean

all: ci

# Everything CI enforces, ordered so the cheapest check fails first.
ci: fmt-check lint test build

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

# Warnings are failures: the point of the gate is that they never accumulate.
lint:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test --all-targets

build:
	cargo build --release

run: build
	./target/release/panther-power

# Re-record the README demo. Needs vhs and ttyd, plus a battery with some discharge
# history — on a machine that has been on mains all day the chart will be empty.
demo: build
	vhs docs/demo.tape

clean:
	cargo clean
