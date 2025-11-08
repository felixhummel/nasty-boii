MAKEFLAGS += --always-make

default:
	cargo fmt --all
	cargo clippy --all-targets
	cargo test
