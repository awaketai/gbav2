build:
	@cargo build

check:
	@cargo check --all

test:
	@cargo nextest run --all-features

fmt:
	@cargo +nightly fmt

lint:
	@cargo clippy -- -D warnings

lint-pedantic:
	@cargo clippy -- -D warnings -W clippy::pedantic

audit:
	@cargo audit

release:
	@cargo release tag --execute
	@git cliff -o CHANGELOG.md
	@git commit -a -n -m "Update CHANGELOG.md" || true
	@git push origin master
	@cargo release push --execute

update-submodule:
	@git submodule update --init --recursive --remote

.PHONY: build check test fmt lint lint-pedantic audit release update-submodule
