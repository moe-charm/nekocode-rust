# 🚀 5-Binary Split Architecture Demo

## Quick Test Commands

```bash
# Build all 5 binaries
cd nekocode-workspace
cargo build --release

# Test each binary
echo "function test() { return 42; }" > /tmp/test.js

# 1. nekocode - Core analysis
./target/release/nekocode analyze /tmp/test.js --stats-only

# 2. nekorefactor - Preview refactoring
./target/release/nekorefactor replace-preview /tmp/test.js "42" "100"

# 3. nekoimpact - Impact analysis
./target/release/nekocode session-create /tmp
./target/release/nekoimpact list

# 4. nekoinc - Incremental analysis
./target/release/nekoinc init <session-id>

# 5. nekomcp - MCP server
./target/release/nekomcp capabilities
```

## Binary Sizes
- nekocode: 16.6MB (includes all Tree-sitter parsers)
- nekorefactor: 2.9MB
- nekoimpact: 2.9MB
- nekoinc: 3.2MB
- nekomcp: 3.7MB

**Each binary is completely standalone!** 🎯