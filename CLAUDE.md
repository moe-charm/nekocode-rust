# ⚠️ CRITICAL INSTRUCTIONS FOR CLAUDE CODE ⚠️

## 🚨 NEVER COMMIT THESE:
1. **test-real-projects/** - Downloaded separately for testing
2. **target/** - Rust build artifacts  
3. **nyash/** - Separate project
4. **build/** - Build artifacts
5. Any file > 10MB (except bin/nekocode_ai)

## ✅ BEFORE ANY git add:
1. Check file size: `du -sh <file>`
2. Check .gitignore is working: `git status --ignored`
3. Repository must stay < 20MB total

## 📏 Size limits:
- Repository total: < 20MB
- Individual files: < 10MB (except binary)
- git clone time: < 5 seconds

Remember: NekoCode's selling point is being LIGHTWEIGHT!
