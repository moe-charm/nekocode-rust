#!/bin/bash
echo "🔍 Checking repository size..."
echo "Git size: $(du -sh .git | cut -f1)"
echo "Working dir: $(du -sh . --exclude=.git | cut -f1)"
echo "Total: $(du -sh . | cut -f1)"

# List large files
echo -e "\n📊 Files > 1MB:"
find . -type f -size +1M -exec du -h {} \; | sort -rh | head -10

# Check ignored files
echo -e "\n🚫 Ignored patterns working:"
git status --ignored --short | head -5
