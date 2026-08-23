# 🚀 NekoCode 5-Binary Toolchain - Modern Makefile
# Unix Philosophy: "Do One Thing and Do It Well" + Instant Usability

.PHONY: all build debug release rust-first-release rust-first-install update-binaries setup test clean clean-all help install quick-test

# 🎯 Default target: Full 5-binary build + setup
all: release

# 🔧 Debug build (5 binaries)
debug:
	@echo "🔧 Building NekoCode 5-binary toolchain (debug)..."
	@cd nekocode-workspace && cargo build
	@echo "✅ Debug build complete!"

# 🏗️ Core build step for 5 binaries
build:
	@echo "🚀 Building NekoCode 5-binary toolchain (release)..."
	@cd nekocode-workspace && cargo build --release
	@echo "✅ Release build complete!"

# 🎊 Full release: build + copy + setup
release: build update-binaries setup
	@echo "🎊 NekoCode 5-binary release complete!"
	@echo "📋 Available commands:"
	@echo "  ./bin/nekocode         - Core analysis engine (16MB)"
	@echo "  ./bin/nekorefactor     - Smart refactoring + strip-comments (2.8MB)"
	@echo "  ./bin/nekoimpact       - Change impact analysis (2.8MB)"
	@echo "  ./bin/nekoinc          - Incremental analysis (3.1MB)"
	@echo "  ./bin/nekomcp          - MCP integration (3.6MB)"

# 🦀 Explicit Rust-first release: stage only the canonical nekocode CLI.
# The historical five-binary targets above remain unchanged for compatibility.
rust-first-release:
	@scripts/update_rust_first_release.sh

# 🦀 Install only the canonical Rust-first CLI wrapper; legacy wrappers are untouched.
rust-first-install:
	@python3 releases/setup.py --install-rust-first

# 📦 Copy all 5 binaries to user-friendly locations
update-binaries: build
	@echo "📦 Copying 5 binaries to bin/ and releases/..."
	@mkdir -p bin/ releases/
	@cp nekocode-workspace/target/release/nekocode bin/
	@cp nekocode-workspace/target/release/nekorefactor bin/
	@cp nekocode-workspace/target/release/nekoimpact bin/
	@cp nekocode-workspace/target/release/nekoinc bin/
	@cp nekocode-workspace/target/release/nekomcp bin/
	@cp nekocode-workspace/target/release/nekocode releases/
	@cp nekocode-workspace/target/release/nekorefactor releases/
	@cp nekocode-workspace/target/release/nekoimpact releases/
	@cp nekocode-workspace/target/release/nekoinc releases/
	@cp nekocode-workspace/target/release/nekomcp releases/
	@chmod +x bin/* releases/*
	@echo "✅ 5 binaries copied and made executable"
	@echo "📊 Sizes:"
	@du -h bin/neko* | sed 's/^/  /'

# 🛠️ Setup scripts for immediate usability
setup:
	@echo "🛠️ Creating setup scripts..."
	@echo '#!/bin/bash' > bin/quick-setup.sh
	@echo 'echo "🐱 NekoCode 5-Binary Toolchain Setup"' >> bin/quick-setup.sh
	@echo 'echo "================================="' >> bin/quick-setup.sh
	@echo 'echo ""' >> bin/quick-setup.sh
	@echo 'echo "📋 Available Commands:"' >> bin/quick-setup.sh
	@echo 'echo "  ./bin/nekocode session-create /path/to/project"' >> bin/quick-setup.sh
	@echo 'echo "  ./bin/nekorefactor strip-comments file.js"' >> bin/quick-setup.sh
	@echo 'echo "  ./bin/nekoimpact analyze SESSION_ID"' >> bin/quick-setup.sh
	@echo 'echo ""' >> bin/quick-setup.sh
	@echo 'echo "🔌 MCP Integration (Claude Code):"' >> bin/quick-setup.sh
	@echo 'echo "  Add to settings: mcp-nekocode-server/mcp_server_real.py"' >> bin/quick-setup.sh
	@echo 'echo ""' >> bin/quick-setup.sh
	@echo 'echo "✅ Ready to use! Run any command with --help for details"' >> bin/quick-setup.sh
	@chmod +x bin/quick-setup.sh
	@echo "✅ Setup script created: bin/quick-setup.sh"

# 🧪 Run all tests
test:
	@echo "🧪 Running workspace tests..."
	@cd nekocode-workspace && cargo test

# 🚀 Quick functionality test of all 5 binaries
quick-test: release
	@echo "⚡ Testing 5-binary functionality..."
	@echo "Testing nekocode..."
	@./bin/nekocode --help >/dev/null && echo "  ✅ nekocode OK" || echo "  ❌ nekocode failed"
	@echo "Testing nekorefactor (with strip-comments)..."
	@./bin/nekorefactor strip-comments --help >/dev/null && echo "  ✅ strip-comments available" || echo "  ❌ strip-comments missing"
	@echo "Testing nekoimpact..."
	@./bin/nekoimpact --help >/dev/null && echo "  ✅ nekoimpact OK" || echo "  ❌ nekoimpact failed"
	@echo "Testing nekoinc..."
	@./bin/nekoinc --help >/dev/null && echo "  ✅ nekoinc OK" || echo "  ❌ nekoinc failed"
	@echo "Testing nekomcp..."
	@./bin/nekomcp --help >/dev/null && echo "  ✅ nekomcp OK" || echo "  ❌ nekomcp failed"
	@echo "🎯 All binaries tested!"

# 🧹 Clean build artifacts (keep bin/ for usability)
clean:
	@echo "🧹 Cleaning build artifacts..."
	@cd nekocode-workspace && cargo clean

# 🧹🔥 Clean everything including bin/ and releases/
clean-all: clean
	@echo "🧹🔥 Cleaning bin/ and releases/ directories..."
	@rm -rf bin/* releases/*
	@echo "✅ Complete cleanup done!"

# 📥 Install all 5 binaries to ~/.local/bin
install: release
	@echo "📥 Installing 5 binaries to ~/.local/bin/..."
	@mkdir -p ~/.local/bin
	@ln -sf $$(pwd)/bin/nekocode ~/.local/bin/nekocode
	@ln -sf $$(pwd)/bin/nekorefactor ~/.local/bin/nekorefactor
	@ln -sf $$(pwd)/bin/nekoimpact ~/.local/bin/nekoimpact
	@ln -sf $$(pwd)/bin/nekoinc ~/.local/bin/nekoinc
	@ln -sf $$(pwd)/bin/nekomcp ~/.local/bin/nekomcp
	@echo "✅ Installed! Available commands:"
	@echo "  nekocode --help"
	@echo "  nekorefactor strip-comments --help"
	@echo "  nekoimpact --help"
	@echo "  nekoinc --help"
	@echo "  nekomcp --help"

# 📖 Help
help:
	@echo "🚀 NekoCode 5-Binary Toolchain - Build Commands"
	@echo ""
	@echo "🎯 Main targets:"
	@echo "  make               - Build all 5 binaries + setup (default)"
	@echo "  make release       - Full release build with setup"
	@echo "  make rust-first-release - Stage only canonical releases/nekocode"
	@echo "  make rust-first-install - Install only the Rust-first CLI wrapper"
	@echo "  make quick-test    - Build and test all binaries"
	@echo ""
	@echo "🔧 Development:"
	@echo "  make debug         - Debug build"
	@echo "  make build         - Release build only"
	@echo "  make update-binaries - Copy binaries to bin/"
	@echo "  make setup         - Create setup scripts"
	@echo "  make test          - Run unit tests"
	@echo ""
	@echo "🧹 Cleanup:"
	@echo "  make clean         - Clean build artifacts (keep bin/)"
	@echo "  make clean-all     - Clean everything including bin/"
	@echo ""
	@echo "📥 Installation:"
	@echo "  make install       - Install to ~/.local/bin/"
	@echo ""
	@echo "🎊 After 'make', use:"
	@echo "  ./bin/nekocode --help"
	@echo "  ./bin/nekorefactor strip-comments --help"
	@echo "  ./bin/quick-setup.sh"
