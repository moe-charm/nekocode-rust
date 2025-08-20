# NekoCode Workspace Makefile
# Build and deploy all binaries with a single command

.PHONY: all build release deploy clean help test

# Default target
all: release deploy

# Help target
help:
	@echo "NekoCode Workspace Build System"
	@echo "================================"
	@echo "  make           - Build release and deploy (default)"
	@echo "  make build     - Build debug version"
	@echo "  make release   - Build release version"
	@echo "  make deploy    - Deploy binaries to all locations"
	@echo "  make clean     - Clean build artifacts"
	@echo "  make test      - Run tests"
	@echo "  make version   - Show version info"
	@echo ""
	@echo "Quick commands:"
	@echo "  make quick     - Fast incremental build + deploy"
	@echo "  make full      - Clean + full rebuild + deploy"

# Build debug version
build:
	@echo "🔧 Building debug version..."
	@cargo build
	@echo "✅ Debug build complete"

# Build release version
release:
	@echo "🚀 Building release version..."
	@cargo build --release
	@echo "✅ Release build complete"

# Deploy to all locations
deploy:
	@echo "📦 Deploying binaries..."
	@./build-and-deploy.sh
	@echo "✅ Deployment complete"

# Quick build and deploy (incremental)
quick:
	@echo "⚡ Quick build and deploy..."
	@cargo build --release
	@./build-and-deploy.sh

# Full rebuild (clean + build + deploy)
full: clean release deploy
	@echo "🎊 Full rebuild complete!"

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@echo "✅ Clean complete"

# Run tests
test:
	@echo "🧪 Running tests..."
	@cargo test
	@echo "✅ Tests complete"

# Show version information
version:
	@echo "📊 NekoCode Version Information:"
	@echo "================================"
	@grep -h "^version" */Cargo.toml | head -1 | cut -d'"' -f2
	@echo ""
	@if [ -f target/release/nekocode ]; then \
		echo "Built binary version:"; \
		./target/release/nekocode --version; \
	else \
		echo "No release binary built yet"; \
	fi

# Install (deploy to system locations)
install: release deploy
	@echo "📋 Installation complete!"
	@echo "Binaries deployed to:"
	@echo "  - ../../releases"
	@echo "  - ../../../../nekocode-cpp-github/nyash/releases"
	@echo "  - ../../../nekocode-cpp-github/releases"