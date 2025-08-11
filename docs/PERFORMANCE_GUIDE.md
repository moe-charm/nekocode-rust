# ⚡ NekoCode Performance Optimization Guide

## 🎯 Quick Reference for Claude Code

### Storage-Optimized Options

| Option | Best For | Performance | Thread Count |
|--------|----------|-------------|--------------|
| `--ssd` | SSD/NVMe drives | 4-16x faster | CPU cores |
| `--hdd` | Mechanical drives | Safe & stable | 1 thread |
| `--progress` | Large projects | Monitoring | Any mode |

### Magic Commands for Claude Code

```bash
# 🔥 For SSD/NVMe systems - Maximum speed
./nekocode_ai analyze large-project/ --ssd --progress

# 🛡️ For HDD systems - Safe processing  
./nekocode_ai analyze large-project/ --hdd --progress

# 📊 Monitor 30K+ files in real-time
tail -f sessions/ai_session_*/progress.txt
```

## 📊 Performance Benchmarks

Based on real-world testing with 38,021 TypeScript files:

| Mode | Time | Speed | Use Case |
|------|------|-------|----------|
| `--hdd` | ~45 min | 1x | Mechanical drives, stability |
| `--ssd` | ~3-8 min | 6-15x | SSD/NVMe, maximum speed |
| Manual threads | Variable | 2-10x | Custom optimization |

## 🚀 Real-World Examples

### Large TypeScript Project (38K files)
```bash
# Monitor analysis progress
./nekocode_ai session-create huge-typescript-project/ --ssd --progress

# Check progress in another terminal  
tail -f sessions/ai_session_20250729_*/progress.txt
# Output: 🚀 Starting analysis: 38,021 files
#         🎉 Analysis complete! 38021 success, 0 errors in 7m 23s
```

### Memory-Constrained System
```bash
# Safe mode for limited resources
./nekocode_ai analyze project/ --hdd --performance
```

### Quick Stats Only
```bash
# Super fast overview (no detailed analysis)
./nekocode_ai analyze project/ --stats-only --ssd
```

## 📈 Performance Impact

### CPU Utilization
- **HDD Mode**: ~25% (single core)  
- **SSD Mode**: ~90-100% (all cores)
- **Auto Mode**: ~90-100% (all cores)

### Memory Usage
- **Base**: ~50MB per thread
- **Large Files**: +10-20MB per complex file
- **Recommendation**: 4GB+ RAM for SSD mode

### Disk I/O Pattern
- **HDD Mode**: Sequential reads (HDD-friendly)
- **SSD Mode**: Random parallel reads (SSD-optimized)

## 🎛️ Advanced Configuration

### Manual Thread Control
```bash
# Custom thread count
./nekocode_ai analyze project/ --threads 4 --progress

# Conservative parallel (good for older SSDs)
./nekocode_ai analyze project/ --threads 2 --progress
```

### Background Processing
```bash
# Run in background with logging
nohup ./nekocode_ai analyze huge-project/ --ssd --progress > analysis.log 2>&1 &

# Monitor from another terminal
tail -f analysis.log
tail -f sessions/ai_session_*/progress.txt
```

## 🔧 Troubleshooting Performance

### Problem: Analysis too slow
**Solution**: 
```bash
# Check storage type first
lsblk | grep -E "(ssd|nvme)"  # If found, use --ssd
./nekocode_ai analyze project/ --ssd --progress
```

### Problem: System becomes unresponsive  
**Solution**:
```bash
# Reduce thread pressure
./nekocode_ai analyze project/ --threads 2 --progress
# Or switch to HDD mode
./nekocode_ai analyze project/ --hdd --progress
```

### Problem: HDD thrashing
**Solution**:
```bash
# Always use HDD mode for mechanical drives
./nekocode_ai analyze project/ --hdd --progress
```

## 📊 Progress Monitoring Features

### Real-Time Progress Display
```
🚀 Starting analysis: 38,021 files in /huge-project
Processing: main.cpp (2.1KB) | 1,234/38,021 (3.2%) | 7m 23s elapsed
Processing: utils.hpp (856B) | 1,235/38,021 (3.2%) | 7m 24s elapsed
🎉 Analysis complete! 38,021 success, 0 errors, 0 skipped in 45m 12s
```

### Progress File Format
```
[2025-07-29 00:15:32] START: 38021 files | Target: /huge-project
[2025-07-29 00:15:33] PROCESSING: 1/38021 (0.0%) | main.cpp (2145) | OK | 1.2s
[2025-07-29 00:15:34] PROCESSING: 2/38021 (0.0%) | utils.hpp (856) | OK | 2.1s
...
[2025-07-29 01:00:44] COMPLETE: 38021/38021 (100%) | Total: 45m 12s | Success: 38021 | Errors: 0 | Skipped: 0
```

## 💡 Claude Code Best Practices

1. **Always start with storage detection**:
   ```bash
   lsblk  # Check if SSD/NVMe available
   ```

2. **For unknown projects, start safe**:
   ```bash
   ./nekocode_ai analyze project/ --hdd --progress
   ```

3. **For known fast storage**:
   ```bash
   ./nekocode_ai analyze project/ --ssd --progress  
   ```

4. **Always monitor large projects**:
   ```bash
   # Essential for 1000+ files
   ./nekocode_ai session-create project/ --progress
   ```

5. **Background processing pattern**:
   ```bash
   nohup ./nekocode_ai analyze project/ --ssd --progress > log.txt 2>&1 &
   tail -f log.txt  # Monitor completion
   ```

---

**Pro Tip**: Start with `--hdd --progress` for safety, then upgrade to `--ssd --progress` once you confirm your system can handle the parallel load!

*Happy Fast Analyzing! 🐱⚡*