# Makefile for the Rust project
PACKAGE  = https://github.com/xbcsmith/antares.git
BINARY   = bin/antares
COMMIT  ?= $(shell git rev-parse --short=16 HEAD)

TOOLS    = $(CURDIR)/tools

# Allow tags to be set on command-line, but don't set them
# by default
override TAGS := $(and $(TAGS),-tags $(TAGS))


V = 0
Q = $(if $(filter 1,$V),,@)
M = $(shell printf "\033[34;1mantares ▶\033[0m")

# Variables
CARGO = cargo
PROJECT_NAME = antares

# Default target
all: build

# Build the project
build: ; $(info $(M) running cargo build...) @ ## Runs a cargo build
	$Q $(CARGO) build

build-linux: ; $(info $(M) running cargo build with target x86_64-unknown-linux-gnu...) @ ## Runs a cargo build with target x86_64-unknown-linux-gnu
	$Q $(CARGO) build --target x86_64-unknown-linux-gnu

build-aarch64: ; $(info $(M) running cargo build with target aarch64-unknown-linux-gnu...) @ ## Runs a cargo build with target aarch64-unknown-linux-gnu
	$Q $(CARGO) build --target aarch64-unknown-linux-gnu

build-windows: ; $(info $(M) running cargo build with target x86_64-pc-windows-msvc...) @ ## Runs a cargo build with target x86_64-pc-windows-msvc
	$Q $(CARGO) build --target x86_64-pc-windows-msvc

build-macos: ; $(info $(M) running cargo build with target x86_64-apple-darwin...) @ ## Runs a cargo build with target x86_64-apple-darwin
	$Q $(CARGO) build --target x86_64-apple-darwin

# Run the project
run: ; $(info $(M) running cargo run...) @ ## Runs a cargo run
	$Q $(CARGO) run

sdk: ; $(info $(M) running  cargo run --package campaign_builder --bin campaign-builder -- --campaign campaigns/tutorial) @ ## Runs a cargo run --package campaign_builder --bin campaign-builder with the tutorial campaign
	$Q $(CARGO) run --package campaign_builder --bin campaign-builder -- --campaign campaigns/tutorial

# Run tests
test: ; $(info $(M) running cargo test...) @ ## Runs a cargo test
	$Q $(CARGO) nextest run --all-features

# Clean the project
clean: ; $(info $(M) running cargo clean...) @ ## Runs a cargo clean
	$Q $(CARGO) clean
	@rm -rvf tools

# Format the code
format: ; $(info $(M) running cargo fmt...) @ ## Runs a cargo fmt
	$Q $(CARGO) fmt --all

# Check for warnings and errors
check: ; $(info $(M) running cargo check...) @ ## Runs a cargo check
	$Q $(CARGO) check --all-targets --all-features

# Lint the code
lint: ; $(info $(M) running cargo clippy...) @ ## Runs a cargo clippy
	$Q $(CARGO) clippy --all-targets --all-features -- -D warnings

install: ; $(info $(M) running cargo install...) @ ## Runs a cargo install
	$Q $(CARGO) install --path .

megalint: ; $(info $(M) running all the things...) @ ## Runs all the things
	$Q $(CARGO) fmt --all \
	    && $(CARGO) check --all-targets --all-features \
		&& $(CARGO) clippy --all-targets --all-features -- -D warnings \
		&& $(CARGO) nextest run --all-features \
		&& $(CARGO) test --doc

# Generate documentation
doc: ; $(info $(M) running cargo doc...) @ ## Runs a cargo doc
	$Q $(CARGO) doc --open

help:
	@grep -E '^[ a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
        awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

.PHONY: all build run sdk test clean format check lint install megalint doc help
