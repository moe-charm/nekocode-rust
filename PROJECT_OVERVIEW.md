# 🚀 NekoCode Project Overview

## 📂 Document Structure

### Core Documents
- **README.md** - Project documentation (English)
- **CLAUDE.md** - Claude-specific context and instructions
- **current_task.md** - Active development task
- **PROJECT_OVERVIEW.md** - This file

### Development Notes
- **completed_tasks.md** - Completed features and fixes
- **technical_notes.md** - Implementation details and decisions
- **archived_docs.md** - Historical documentation reference

## 🎯 Project Status

### Current Focus
**Smart Refactoring with Tree-sitter** - Adding AST-based precise code manipulation

### Recent Achievements
- ✅ 5-binary Unix toolchain architecture implemented
- ✅ Immediate application as default (Git as safety net)
- ✅ MCP integration for Claude Code
- ✅ 16x performance improvement with Tree-sitter

### Known Issues
- 🐛 Semantic positioning inaccuracy (being fixed with Smart commands)
- 🐛 move-lines command UX issues
- 🐛 MCP server caching delays

## 🏗️ Architecture

### Binary Structure (5-tool Unix Philosophy)
```
nekocode       - Core analysis engine (Tree-sitter)
nekorefactor   - Code refactoring tool
nekoimpact     - Change impact analysis
nekoinc        - Incremental analysis
nekomcp        - MCP integration gateway
```

### Key Technologies
- **Language**: Rust
- **Parser**: Tree-sitter (7 languages)
- **Async**: Tokio
- **CLI**: Clap
- **Integration**: MCP (Model Context Protocol)

## 🔄 Development Workflow

### For Contributors
1. Check `current_task.md` for active work
2. Review `technical_notes.md` for implementation details
3. Test in `/test-workspace/` directory (Git-ignored)
4. Update relevant documentation

### For Claude
1. Primary focus: `current_task.md`
2. Context: `CLAUDE.md`
3. Technical details: `technical_notes.md`
4. Test location: `../test-workspace/`

## 📊 Performance Metrics

- **Analysis Speed**: 1.2s (vs 19.5s PEGTL)
- **Incremental Update**: 23-49ms (918-1956x speedup)
- **Languages**: 8 (JS/TS/Python/Rust/C++/C#/Go/C)
- **Binary Size**: ~250MB total (5 binaries)

## 🎯 Roadmap

### Immediate (This Week)
- [ ] Smart refactoring with Tree-sitter AST
- [ ] Language-specific rules (Python, TypeScript, Rust)

### Short Term (This Month)
- [ ] Auto-indent detection
- [ ] Symbol-based navigation
- [ ] Batch operations

### Long Term
- [ ] AI-assisted refactoring
- [ ] Cross-project analysis
- [ ] Cloud-based analysis cluster

---

**Last Updated**: 2025-08-17
**Maintainer**: Claude + User Collaboration
**Repository**: github.com/moe-charm/nekocode-rust