# Repository layout

Status: canonical after physical legacy retirement, 2026-08-23.

```text
nekocode-workspace/
├── nekocode-core/       # snapshot/context semantics and tests
└── nekocode/            # canonical two-command CLI
mcp-nekocode-server/     # thin two-tool stdio adapter
schemas/                 # snapshot-v1/context-v1 contracts
skills/                  # workflow and stop conditions
docs/                    # product and trust decisions
Dockerfile               # canonical CLI + local MCP image
```

The dependency direction is one-way: CLI consumes core. MCP invokes the
canonical CLI and must return the same core payload. Schema and Skill files do
not implement analysis rules.

Use:

```bash
make verify
# or
cd nekocode-workspace
cargo test
cargo check --all-targets
```

The retired implementation is absent from `main`. Its recovery points are
`legacy-multilang-final` and `archive/legacy-multilang-final`; see
[legacy-retirement.md](legacy-retirement.md).
