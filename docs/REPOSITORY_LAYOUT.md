# Repository layout

The user-approved 2026-09-08 [Symbol context v1](symbol-context-v1.md) redesign
adds LSP-backed investigation and saved continuation. It supersedes prior
backend-deferral statements; existing snapshot-v1/context-v1 remain compatible.

Status: core split and symbol investigation implemented, 2026-09-08.

```text
nekocode-workspace/
├── nekocode-core/       # snapshot/context semantics and tests
│   ├── src/rust_context/ # Cargo/Git providers, execution, budgets, summary
│   └── src/symbol_context/ # external LSP, captured evidence, saved follow-up
└── nekocode/            # canonical two-command CLI
mcp-nekocode-server/     # thin two-tool stdio adapter
schemas/                 # versioned snapshot/Git/symbol/delta contracts
skills/                  # workflow and stop conditions
docs/                    # product and trust decisions
Dockerfile               # canonical CLI + local MCP image
```

The dependency direction is one-way: CLI consumes core. MCP invokes the
canonical CLI and must return the same core payload. Schema and Skill files do
not implement analysis rules.

The existing public facade holds orchestration, stable models and re-exports;
the provider modules are described in [module-boundaries.md](module-boundaries.md).
This is a mechanical responsibility split, not a new parser or language
abstraction.

Use:

```bash
make verify
# or
python3 -m pip install -r requirements-dev.txt
cd nekocode-workspace
cargo test --locked
cargo check --locked --all-targets
```

The retired implementation is absent from `main`. Its recovery points are
`legacy-multilang-final` and `archive/legacy-multilang-final`; see
[legacy-retirement.md](legacy-retirement.md).
