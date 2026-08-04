# Development tasks.
#
# CI runs these same recipes, so what passes here is what passes there — the gate list
# lives in one place rather than being duplicated into the workflow.

# List the recipes
default:
    @just --list

# Everything CI enforces, cheapest check first
ci: fmt-check lint test build

# Format in place
fmt:
    cargo fmt --all

# Fail if anything is unformatted
fmt-check:
    cargo fmt --all --check

# Warnings are failures, so they can never accumulate
lint:
    cargo clippy --all-targets -- -D warnings

# Unit and rendering tests
test:
    cargo test --all-targets

# Optimised binary
build:
    cargo build --release

# Build and launch
run: build
    ./target/release/panther-power

# Recording needs vhs and ttyd, plus a battery with some discharge history: on a machine
# that has been on mains all day the chart has nothing to draw.

# Re-record docs/demo.gif
demo: build
    vhs docs/demo.tape

# Remove build artifacts
clean:
    cargo clean
