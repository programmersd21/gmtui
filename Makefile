APP_NAME := gmtui
CARGO := cargo

.PHONY: all fmt fmt-check lint test build build-release run run-release clean check clippy ci

all: fmt lint test build

fmt:
	$(CARGO) fmt

fmt-check:
	$(CARGO) fmt -- --check

lint:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

test:
	$(CARGO) test --all-features

check:
	$(CARGO) check --all-features

build:
	$(CARGO) build

build-release:
	$(CARGO) build --release

run:
	$(CARGO) run

run-release:
	./target/release/$(APP_NAME)

clean:
	$(CARGO) clean

clippy:
	$(CARGO) clippy --all-targets --all-features

ci: fmt-check lint test build-release
