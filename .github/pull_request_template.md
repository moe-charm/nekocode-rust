# 🐱 NekoCode Pull Request

## 📋 Description
<!-- Provide a clear and concise description of your changes -->



## 🎯 Type of Change
<!-- Mark the relevant option with an [x] -->

- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)
- [ ] ✨ New feature (non-breaking change that adds functionality)
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] 🔧 Maintenance (dependency updates, refactoring, etc.)
- [ ] 📚 Documentation update
- [ ] 🚀 Performance improvement
- [ ] 🧪 Test addition/improvement

## 🔗 Related Issues
<!-- Link to any related issues using "Fixes #123" or "Closes #123" -->

- Fixes #
- Related to #

## 🧪 Testing
<!-- Describe how you tested your changes -->

### Test Steps
1. 
2. 
3. 

### Test Files/Commands Used
```bash
# Example test commands
make verify
cargo run --manifest-path nekocode-workspace/Cargo.toml -p nekocode -- snapshot nekocode-workspace
```

### Contract Surface Tested

- [ ] snapshot
- [ ] context
- [ ] MCP parity
- [ ] schema validation
- [ ] execution-trust boundary

## 📊 Performance Impact
<!-- If applicable, describe any performance implications -->

- [ ] No performance impact
- [ ] Performance improvement (describe below)
- [ ] Potential performance regression (describe below)

**Details:**


## 🔄 Breaking Changes
<!-- If this is a breaking change, describe what breaks and how to migrate -->

- [ ] No breaking changes
- [ ] Breaking changes (describe below)

**Migration Guide:**


## ✅ Checklist
<!-- Ensure all items are completed before submitting -->

### Code Quality
- [ ] My code follows the project's style guidelines
- [ ] I have performed a self-review of my own code
- [ ] My code has proper error handling
- [ ] I have commented my code where necessary
- [ ] My changes generate no new warnings

### Testing
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] I have tested with real-world code samples
- [ ] I have tested edge cases and error conditions

### Documentation
- [ ] I have updated relevant documentation
- [ ] I have updated CLI help text if applicable
- [ ] I have updated example commands if applicable
- [ ] Any new features are documented

### Security & Dependencies
- [ ] My changes don't introduce security vulnerabilities
- [ ] I have not added unnecessary dependencies
- [ ] Any new dependencies are justified and documented

## 📝 Additional Notes
<!-- Any additional information, context, or screenshots -->



## 📋 Reviewer Focus Areas
<!-- Help reviewers by highlighting what to focus on -->

- [ ] Code correctness
- [ ] Performance implications
- [ ] Security considerations
- [ ] Breaking change validation
- [ ] Documentation accuracy
- [ ] Test coverage

**Specific areas to review:**


---

🐱 **Thank you for contributing to NekoCode!**

<!-- 
For maintainers: This PR will trigger the following automated checks:
- Rust and MCP tests
- Artifact contract checks
- Execution-safety fixtures
-->
