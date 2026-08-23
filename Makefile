.PHONY: all build check test verify install package docker clean help

MANIFEST := nekocode-workspace/Cargo.toml

all: check

build:
	cargo build --manifest-path $(MANIFEST) -p nekocode --release

check:
	cargo check --manifest-path $(MANIFEST) --all-targets

test:
	cargo test --manifest-path $(MANIFEST)
	python3 -m unittest discover -s mcp-nekocode-server/tests -p 'test_*.py'

verify: check test
	python3 -c 'import json; json.load(open("schemas/snapshot-v1.schema.json")); json.load(open("schemas/context-v1.schema.json"))'

install:
	cargo install --path nekocode-workspace/nekocode --locked

package:
	scripts/update_rust_first_release.sh

docker:
	docker build -t nekocode .

clean:
	cargo clean --manifest-path $(MANIFEST)

help:
	@echo "make build    Build the canonical nekocode CLI"
	@echo "make check    Check core and CLI"
	@echo "make test     Run Rust and MCP tests"
	@echo "make verify   Run all local verification"
	@echo "make install  Install nekocode with Cargo"
	@echo "make package  Stage one CLI binary under dist/"
	@echo "make docker   Build the local MCP image"
