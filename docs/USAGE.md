# 🐱 NekoCode Usage Guide

## 📖 Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Basic Usage](#basic-usage)
4. [Advanced Features](#advanced-features)
5. [⚡ Incremental Analysis](#incremental-analysis) ⭐ **NEW!**
6. [AI Developer Guide](#ai-developer-guide)
7. [Troubleshooting](#troubleshooting)

## Introduction

NekoCode C++ is a lightning-fast code analysis tool designed for modern development workflows, especially for AI-assisted development with tools like Claude Code and GitHub Copilot.

## Installation

### Prerequisites
- C++17 compatible compiler (GCC 7+, Clang 5+, MSVC 2017+)
- CMake 3.10 or higher
- Git

### Build Instructions

```bash
# 1. Clone the repository
git clone https://github.com/moe-charm/nekocode.git
cd nekocode

# 2. Create build directory
mkdir build && cd build

# 3. Configure with CMake
cmake ..

# 4. Build (parallel build recommended)
make -j$(nproc)

# 5. Verify installation (binary is placed under bin/)
./bin/nekocode_ai --help
```

## Basic Usage

### Single File Analysis

```bash
# Analyze a C++ file
./bin/nekocode_ai main.cpp

# Analyze a JavaScript file
./bin/nekocode_ai app.js

# With performance statistics
./bin/nekocode_ai --performance main.cpp
```

### Directory Analysis

```bash
# Analyze entire src directory
./bin/nekocode_ai src/

# Analyze specific language only
./bin/nekocode_ai --lang cpp src/

# Compact JSON output
./bin/nekocode_ai --compact src/
```

## Advanced Features

### ⚡ Performance Optimization (NEW!)

NekoCode now includes intelligent storage optimization for lightning-fast analysis:

```bash
# 🔥 SSD Mode - Maximum parallel performance
./bin/nekocode_ai analyze large-project/ --ssd --performance
# Uses all CPU cores, perfect for NVMe/SSD storage

# 🛡️ HDD Mode - Safe sequential processing  
./bin/nekocode_ai analyze large-project/ --hdd --performance
# Single thread, prevents HDD thrashing

# 📊 Progress Display - Monitor large projects
./bin/nekocode_ai session-create large-project/ --progress
# Real-time progress: "🚀 Starting analysis: 38,021 files"
# Creates progress file: sessions/SESSION_ID_progress.txt
```

**Claude Code Pro Tip**: For projects with 30,000+ files, always use `--progress` to monitor completion!

### Interactive Sessions

The most powerful feature of NekoCode!

```bash
# 1. Create a session with progress monitoring
./bin/nekocode_ai session-create /path/to/your/project --progress
# Output: Session created! Session ID: ai_session_20250727_180532

# 2. Run various analyses using session ID
SESSION_ID=ai_session_20250727_180532

# Project statistics
./bin/nekocode_ai session-command $SESSION_ID stats

# Complexity ranking (most important!)
./bin/nekocode_ai session-command $SESSION_ID complexity

# File search
./bin/nekocode_ai session-command $SESSION_ID "find manager"

# Function structure analysis
./bin/nekocode_ai session-command $SESSION_ID structure
```

### C++ Specific Analysis

#### Include Dependencies

```bash
# Generate dependency graph
./bin/nekocode_ai session-command $SESSION_ID include-graph

# Detect circular dependencies (critical!)
./bin/nekocode_ai session-command $SESSION_ID include-cycles

# Find unused includes
./bin/nekocode_ai session-command $SESSION_ID include-unused
```

#### Template & Macro Analysis

```bash
# Detect template specializations
./bin/nekocode_ai session-command $SESSION_ID template-analysis

# Track macro expansions
./bin/nekocode_ai session-command $SESSION_ID macro-analysis

# Detect metaprogramming patterns
./bin/nekocode_ai session-command $SESSION_ID metaprogramming
```

## AI Developer Guide

### Using with Claude Code

1. **Place NekoCode in your project**
   ```bash
   cd your-project
   git clone https://github.com/moe-charm/nekocode.git tools/nekocode
   ```

2. **Magic words to tell Claude**
   ```
   "There's a code analysis tool in tools/nekocode"
   "Measure the complexity of this project"
   "Check for circular dependencies"
   ```

3. **Claude automatically**
   - Builds the tool
   - Creates a session
   - Runs analysis
   - Interprets results

### Practical Example: Refactoring

```bash
# 1. Measure current complexity
./bin/nekocode_ai session-command $SESSION_ID complexity

# Output example:
# FileA.cpp: Complexity 156 (Very Complex)
# FileB.cpp: Complexity 89 (Complex)

# 2. Perform refactoring

# 3. Verify improvements
./nekocode_ai session-command $SESSION_ID complexity
# FileA.cpp: Complexity 23 (Simple)  ← 85% reduction!
```

## Troubleshooting

### Build Issues

**Q: CMake says C++17 is not supported**
```bash
# Check GCC version
g++ --version

# If old, specify newer compiler
cmake -DCMAKE_CXX_COMPILER=g++-9 ..
```

**Q: Do I need Tree-sitter?**
```text
No. Tree-sitter is integrated as a lightweight placeholder in this project,
so the build works without installing Tree-sitter. There is no CMake toggle
to disable/enable it; PEGTL is the primary parser.
```

### Runtime Issues

**Q: Session not found**
```bash
# List available sessions
ls sessions/

# Create new session
./bin/nekocode_ai session-create .
```

**Q: Out of memory**
```bash
# Use HDD mode (single thread)
./bin/nekocode_ai analyze large-project/ --hdd

# Manual thread limit
./bin/nekocode_ai --threads 2 large-project/

# Stats only mode
./bin/nekocode_ai --stats-only large-project/
```

**Q: Analysis taking too long**
```bash
# For SSD/NVMe storage - use parallel mode
./bin/nekocode_ai analyze project/ --ssd --progress

# Monitor progress in real-time
tail -f sessions/SESSION_ID_progress.txt

# Check what's happening
./bin/nekocode_ai analyze project/ --performance
```

**Q: HDD getting hammered**
```bash
# Safe HDD mode (single thread, sequential)
./bin/nekocode_ai analyze project/ --hdd --progress

# This prevents disk thrashing on mechanical drives
```

## 💡 Pro Tips

1. **Storage-Aware Analysis**: 
   - Use `--ssd` for SSD/NVMe drives (4-16x faster!)
   - Use `--hdd` for mechanical drives (safe & stable)
   - Always add `--progress` for projects with 1000+ files

2. **Complexity First**: Always start with `complexity` command to identify problem files

3. **Use Sessions**: For repeated analysis, always use sessions (180x faster!)

4. **Large Project Strategy**:
   ```bash
   # Perfect workflow for 30K+ files
   ./nekocode_ai session-create huge-project/ --ssd --progress
   tail -f sessions/ai_session_*/progress.txt  # Monitor progress
   ```

5. **Parallel Build**: Use `make -j$(nproc)` to utilize all cores

6. **JSON Output**: Use `--compact` option for integration with other tools

---

## ⚡ Incremental Analysis

**Revolutionary ultra-fast code analysis for iterative development!** 🚀

### What is Incremental Analysis?

Instead of re-analyzing your entire project every time you make changes, NekoCode's incremental analysis only processes files that have actually changed. This results in **918-1956x speedup** for typical development workflows.

### Quick Start

```bash
# 1. Create a session (one-time setup)
./nekocode-rust session-create src/
# Output: Session created: abc12345 (analyzed 85 files in 267ms)

# 2. Make code changes
vim src/main.rs    # Edit your code
vim src/lib.rs     # Edit another file

# 3. Update only changed files
./nekocode-rust session-update abc12345
# Output: Updated 2 files in 23ms (1956.5x speedup)
```

### Command Reference

#### Basic Commands
```bash
# Create session
./nekocode-rust session-create <directory>

# Update changed files
./nekocode-rust session-update <session_id>

# Preview changes without updating
./nekocode-rust session-update <session_id> --dry-run

# Get detailed JSON output
./nekocode-rust session-update <session_id> --verbose
```

#### Example Outputs

**Verbose Mode Output:**
```json
{
  "performance": {
    "analysis_time": "49ms",
    "speedup": "918.4x",
    "status": "updated"
  },
  "session_id": "abc12345",
  "summary": {
    "added_files": 0,
    "changed_files": 1,
    "deleted_files": 0,
    "total_files": 85,
    "estimated_speedup": 918.3673469387755
  }
}
```

**Dry-run Mode Output:**
```
Session abc12345 has pending changes:
Total files in session: 85

  M main.rs
  M lib.rs

Summary: 0 added, 2 modified, 0 deleted
Run without --dry-run to apply these changes
```

### Performance Results (Real Production Testing)

| Scenario | Project Size | Initial Analysis | Incremental Update | Speedup |
|----------|--------------|------------------|-------------------|---------|
| **nyash project** | 85 files | 267ms | 23-49ms | **918-1956x** |
| **Small change** | 85 files | 267ms | 23ms | **1956x faster** |
| **Multiple files** | 85 files | 267ms | 49ms | **918x faster** |
| **Change detection** | Any size | N/A | < 1ms | **45000x faster** |

### Development Workflow Examples

#### Example 1: Rapid Iteration
```bash
# Setup (once)
./nekocode-rust session-create my-project/
# Session 4a7b2c89 created in 1.2s

# Development loop
vim src/api.js                            # Edit
./nekocode-rust session-update 4a7b2c89   # 25ms update
vim src/utils.js                          # Edit
./nekocode-rust session-update 4a7b2c89   # 31ms update
vim src/main.js                           # Edit  
./nekocode-rust session-update 4a7b2c89   # 28ms update

# Total time: 84ms vs 3600ms (42x faster for 3 iterations)
```

#### Example 2: Pre-commit Verification
```bash
# Quick check before committing
./nekocode-rust session-update my-session --dry-run
# Shows: "3 files changed, would analyze: main.rs, lib.rs, utils.rs"

# Commit with confidence
git add -A && git commit -m "Feature complete"

# Update session to match commit
./nekocode-rust session-update my-session
# "3 files updated in 45ms (1200x speedup)"
```

#### Example 3: Large Project Development
```bash
# Enterprise project with 500+ files
./nekocode-rust session-create enterprise-app/
# Session created in 2.3s (500 files analyzed)

# Daily development - only modified files get re-analyzed
./nekocode-rust session-update my-session --verbose
# {
#   "performance": {"analysis_time": "67ms", "speedup": "2088x"},
#   "summary": {"changed_files": 3, "total_files": 500}
# }
```

### How It Works

1. **Change Detection**: NekoCode tracks file modification times and content hashes
2. **Smart Filtering**: Only files with actual changes are re-analyzed
3. **Session Persistence**: Analysis results are cached between runs
4. **Instant Feedback**: File change detection happens in < 1ms

### Supported Languages

Incremental analysis works with all supported languages:
- ✅ **JavaScript/TypeScript** 
- ✅ **Python**
- ✅ **C/C++**
- ✅ **C#**
- ✅ **Go**
- ✅ **Rust**

### Integration with AI Development

Perfect for AI-assisted development workflows:

```bash
# Claude Code workflow
./nekocode-rust session-create project/        # Initial setup
# ... Claude makes code changes ...
./nekocode-rust session-update session-id      # Instant analysis
# ... Claude sees results and continues ...
```

### Troubleshooting

**Q: Session not updating?**
```bash
# Check session status
./nekocode-rust session-list
# Verify files were actually modified
./nekocode-rust session-update session-id --dry-run
```

**Q: Performance not as expected?**
```bash
# Check if files are being detected as changed
./nekocode-rust session-update session-id --verbose
# Look for "changed_files" count in output
```

**Q: Want to force full re-analysis?**
```bash
# Delete session and recreate
./nekocode-rust session-delete session-id
./nekocode-rust session-create project/
```

---

For more information, visit the [official documentation](https://github.com/moe-charm/nekocode-rust)!

*Happy Analyzing! 🐱*
