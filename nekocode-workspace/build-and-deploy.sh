#!/bin/bash
# 🚀 NekoCode Workspace Auto-Build and Deploy Script
# Builds all binaries and automatically deploys to multiple locations

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 Building NekoCode Workspace (5 binaries)...${NC}"

# Build all binaries
if cargo build --release; then
    echo -e "${GREEN}✅ Build successful!${NC}"
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

# Define binaries
BINARIES=(
    "nekocode"
    "nekorefactor"
    "nekoimpact"
    "nekoinc"
    "nekomcp"
)

# Define deployment locations
DEPLOY_LOCATIONS=(
    "../releases"                              # IMPORTANT: nekocode-rust-clean/releases (for git clone users!)
    "../../releases"                           # nekocode-rust-clean/releases (backup)
    "../../../../nekocode-cpp-github/nyash/releases"  # nyash/releases
    "../../../nekocode-cpp-github/releases"    # main releases (if exists)
)

# Deploy to each location
for location in "${DEPLOY_LOCATIONS[@]}"; do
    if [ -d "$location" ] || mkdir -p "$location" 2>/dev/null; then
        echo -e "${YELLOW}📂 Deploying to $location${NC}"
        
        for binary in "${BINARIES[@]}"; do
            if [ -f "target/release/$binary" ]; then
                cp -f "target/release/$binary" "$location/$binary"
                chmod +x "$location/$binary"
                # Verify the copy was successful
                if [ -f "$location/$binary" ]; then
                    echo -e "${GREEN}  ✓ $binary copied successfully${NC}"
                else
                    echo -e "${RED}  ✗ Failed to copy $binary${NC}"
                fi
            else
                echo -e "${RED}  ✗ $binary not found${NC}"
            fi
        done
    else
        echo -e "${YELLOW}⚠️  Skipping $location (not accessible)${NC}"
    fi
done

# Test the main binary
echo -e "\n${BLUE}🧪 Testing nekocode binary...${NC}"
if ./target/release/nekocode --version; then
    echo -e "${GREEN}✅ nekocode working correctly!${NC}"
    
    # Show binary sizes
    echo -e "\n${BLUE}📊 Binary sizes:${NC}"
    for binary in "${BINARIES[@]}"; do
        if [ -f "target/release/$binary" ]; then
            size=$(du -h "target/release/$binary" | cut -f1)
            echo -e "  $binary: ${GREEN}$size${NC}"
        fi
    done
    
    echo -e "\n${GREEN}🎊 Deployment complete!${NC}"
    echo -e "${BLUE}💡 All binaries have been deployed to:${NC}"
    for location in "${DEPLOY_LOCATIONS[@]}"; do
        if [ -d "$location" ]; then
            echo -e "  ${GREEN}✓${NC} $location"
        fi
    done
else
    echo -e "${RED}❌ Binary test failed${NC}"
    exit 1
fi