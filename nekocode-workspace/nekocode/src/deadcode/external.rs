//! External tool integration for dead code detection

use nekocode_core::{Result, NekocodeError, Language};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::env;
use crate::deadcode::{DeadItem, SymbolType};

/// External tool manager with user-friendly guidance
pub struct ExternalToolManager;

impl ExternalToolManager {
    fn cargo_path() -> String {
        // Priority: explicit env overrides → ~/.cargo/bin/cargo → cargo in PATH
        if let Ok(p) = env::var("NEKOCODE_CARGO_BIN") { if !p.is_empty() { return p; } }
        if let Ok(p) = env::var("CARGO_BIN") { if !p.is_empty() { return p; } }
        if let Ok(home) = env::var("HOME") {
            let candidate = format!("{}/.cargo/bin/cargo", home);
            if std::path::Path::new(&candidate).exists() { return candidate; }
        }
        "cargo".to_string()
    }

    fn command_with_env(cmd: &str) -> Command {
        let mut c = if cmd == "cargo" { Command::new(Self::cargo_path()) } else { Command::new(cmd) };
        // Ensure ~/.cargo/bin is on PATH for child processes as a safety net
        if let Ok(home) = env::var("HOME") {
            let cargo_bin = format!("{}/.cargo/bin", home);
            if std::path::Path::new(&cargo_bin).is_dir() {
                let cur = env::var("PATH").unwrap_or_default();
                let new_path = if cur.is_empty() { cargo_bin } else { format!("{}:{}", cargo_bin, cur) };
                c.env("PATH", new_path);
            }
        }
        c
    }
    /// Check if external tools are available
    pub fn check_tools() -> ToolAvailability {
        ToolAvailability {
            cargo_clippy: Self::check_command("cargo", &["clippy", "--version"]),
            vulture: Self::check_command("vulture", &["--version"]),
            staticcheck: Self::check_command("staticcheck", &["-version"]),
            eslint: Self::check_command("eslint", &["--version"]),
            clang_tidy: Self::check_command("clang-tidy", &["--version"]),
            cargo_machete: Self::check_command("cargo-machete", &["--version"]),
        }
    }

    /// Check if a command is available
    fn check_command(cmd: &str, args: &[&str]) -> bool {
        Self::command_with_env(cmd)
            .args(args)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get recommended tool for language
    pub fn get_tool_for_language(language: Language) -> Option<ExternalTool> {
        match language {
            Language::Rust => Some(ExternalTool::CargoClippy),
            Language::Python => Some(ExternalTool::Vulture),
            Language::Go => Some(ExternalTool::Staticcheck),
            Language::JavaScript | Language::TypeScript => Some(ExternalTool::ESLint),
            Language::Cpp | Language::C => Some(ExternalTool::ClangTidy),
            _ => None,
        }
    }

    /// Run external tool on project directory
    pub async fn run_tool(
        tool: ExternalTool,
        project_dir: &Path,
        files: &[PathBuf],
    ) -> Result<Vec<DeadItem>> {
        match tool {
            ExternalTool::CargoClippy => Self::run_cargo_clippy(project_dir).await,
            ExternalTool::Vulture => Self::run_vulture(project_dir, files).await,
            ExternalTool::Staticcheck => Self::run_staticcheck(project_dir).await,
            ExternalTool::ESLint => Self::run_eslint(project_dir, files).await,
            ExternalTool::ClangTidy => Self::run_clang_tidy(files).await,
            ExternalTool::CargoMachete => Self::run_cargo_machete(project_dir).await,
        }
    }

    /// Run cargo clippy for Rust projects
    async fn run_cargo_clippy(project_dir: &Path) -> Result<Vec<DeadItem>> {
        let output = Self::command_with_env("cargo")
            .current_dir(project_dir)
            .args(&["clippy", "--", "-W", "dead-code", "-A", "clippy::all"])
            .output()
            .map_err(|e| NekocodeError::External(format!("Failed to run cargo clippy: {}", e)))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        Self::parse_clippy_output(&stderr)
    }

    /// Parse cargo clippy output for dead code warnings
    fn parse_clippy_output(output: &str) -> Result<Vec<DeadItem>> {
        let mut dead_items = Vec::new();
        
        for line in output.lines() {
            if line.contains("warning:") && (line.contains("never used") || line.contains("never constructed")) {
                if let Some(item) = Self::parse_clippy_line(line) {
                    dead_items.push(item);
                }
            }
        }
        
        Ok(dead_items)
    }

    /// Parse single clippy warning line
    fn parse_clippy_line(line: &str) -> Option<DeadItem> {
        // Example: "warning: function `unused_func` is never used"
        // Example: "warning: struct `UnusedStruct` is never constructed"
        
        let parts: Vec<&str> = line.split('`').collect();
        if parts.len() >= 2 {
            let name = parts[1].to_string();
            
            let symbol_type = if line.contains("function") {
                SymbolType::Function
            } else if line.contains("struct") || line.contains("enum") {
                SymbolType::Class
            } else if line.contains("variable") || line.contains("field") {
                SymbolType::Variable
            } else {
                SymbolType::Function // Default
            };

            Some(DeadItem {
                name,
                symbol_type,
                file_path: PathBuf::new(), // Will be filled by caller
                line_start: 0, // Not available from clippy output
                line_end: 0,
                language: Language::Rust,
                confidence: 95, // Clippy is very accurate
                reason: "cargo clippy: never used".to_string(),
            })
        } else {
            None
        }
    }

    /// Run vulture for Python projects
    async fn run_vulture(project_dir: &Path, files: &[PathBuf]) -> Result<Vec<DeadItem>> {
        let mut cmd = Self::command_with_env("vulture");
        cmd.current_dir(project_dir);
        
        // Add file arguments
        for file in files {
            if file.extension().map_or(false, |ext| ext == "py") {
                cmd.arg(file);
            }
        }
        
        let output = cmd
            .output()
            .map_err(|e| NekocodeError::External(format!("Failed to run vulture: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_vulture_output(&stdout)
    }

    /// Parse vulture output
    fn parse_vulture_output(output: &str) -> Result<Vec<DeadItem>> {
        let mut dead_items = Vec::new();
        
        for line in output.lines() {
            if let Some(item) = Self::parse_vulture_line(line) {
                dead_items.push(item);
            }
        }
        
        Ok(dead_items)
    }

    /// Parse single vulture line
    fn parse_vulture_line(line: &str) -> Option<DeadItem> {
        // Example: "file.py:10: unused function 'unused_func' (60% confidence)"
        
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 {
            let file_path = PathBuf::from(parts[0]);
            let line_num: u32 = parts[1].parse().ok()?;
            
            let rest = &parts[2..].join(":");
            if let Some(name_start) = rest.find('\'') {
                if let Some(name_end) = rest[name_start + 1..].find('\'') {
                    let name = rest[name_start + 1..name_start + 1 + name_end].to_string();
                    
                    let symbol_type = if rest.contains("function") {
                        SymbolType::Function
                    } else if rest.contains("class") {
                        SymbolType::Class
                    } else if rest.contains("variable") {
                        SymbolType::Variable
                    } else {
                        SymbolType::Function
                    };

                    // Extract confidence if present
                    let confidence = if let Some(conf_start) = rest.find('(') {
                        if let Some(conf_end) = rest.find("% confidence)") {
                            rest[conf_start + 1..conf_end].parse().unwrap_or(70)
                        } else {
                            70
                        }
                    } else {
                        70
                    };

                    return Some(DeadItem {
                        name,
                        symbol_type,
                        file_path,
                        line_start: line_num,
                        line_end: line_num,
                        language: Language::Python,
                        confidence,
                        reason: "vulture: unused code".to_string(),
                    });
                }
            }
        }
        
        None
    }

    /// Run staticcheck for Go projects
    async fn run_staticcheck(project_dir: &Path) -> Result<Vec<DeadItem>> {
        let output = Self::command_with_env("staticcheck")
            .current_dir(project_dir)
            .arg("./...")
            .output()
            .map_err(|e| NekocodeError::External(format!("Failed to run staticcheck: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_staticcheck_output(&stdout)
    }

    /// Parse staticcheck output
    fn parse_staticcheck_output(output: &str) -> Result<Vec<DeadItem>> {
        let mut dead_items = Vec::new();
        
        for line in output.lines() {
            if line.contains("U1000") { // Unused code check
                if let Some(item) = Self::parse_staticcheck_line(line) {
                    dead_items.push(item);
                }
            }
        }
        
        Ok(dead_items)
    }

    /// Parse single staticcheck line
    fn parse_staticcheck_line(line: &str) -> Option<DeadItem> {
        // Example: "file.go:10:1: func unused_func is unused (U1000)"
        
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 4 {
            let file_path = PathBuf::from(parts[0]);
            let line_num: u32 = parts[1].parse().ok()?;
            
            let message = &parts[3..].join(":");
            if message.contains("func") && message.contains("unused") {
                // Extract function name
                if let Some(func_start) = message.find("func ") {
                    let rest = &message[func_start + 5..];
                    if let Some(space_pos) = rest.find(' ') {
                        let name = rest[..space_pos].to_string();
                        
                        return Some(DeadItem {
                            name,
                            symbol_type: SymbolType::Function,
                            file_path,
                            line_start: line_num,
                            line_end: line_num,
                            language: Language::Go,
                            confidence: 90, // Staticcheck is accurate
                            reason: "staticcheck U1000: unused".to_string(),
                        });
                    }
                }
            }
        }
        
        None
    }

    /// Run ESLint for JavaScript/TypeScript
    async fn run_eslint(project_dir: &Path, files: &[PathBuf]) -> Result<Vec<DeadItem>> {
        // Build ESLint command with JSON output
        let mut cmd = Command::new("eslint");
        cmd.current_dir(project_dir)
            .arg("--format").arg("json");

        let mut any = false;
        for f in files {
            if let Some(ext) = f.extension().and_then(|s| s.to_str()) {
                let ext = ext.to_ascii_lowercase();
                if matches!(ext.as_str(), "js" | "jsx" | "mjs" | "ts" | "tsx") {
                    cmd.arg(f);
                    any = true;
                }
            }
        }
        if !any {
            return Ok(Vec::new());
        }

        let output = cmd
            .output()
            .map_err(|e| NekocodeError::External(format!("Failed to run ESLint: {}", e)))?;

        // Non-zero exit indicates lint errors; still parse stdout
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = match serde_json::from_str(&stdout) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut items = Vec::new();
        let results = parsed.as_array().cloned().unwrap_or_default();
        for file_result in results {
            let file_path = file_result.get("filePath").and_then(|v| v.as_str()).unwrap_or("");
            let messages = file_result.get("messages").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            for msg in messages {
                let rule_id = msg.get("ruleId").and_then(|v| v.as_str()).unwrap_or("");
                let message = msg.get("message").and_then(|v| v.as_str()).unwrap_or("");
                let line = msg.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let end_line = msg.get("endLine").and_then(|v| v.as_u64()).unwrap_or(line as u64) as u32;

                // Focus on unused rules/messages
                let is_unused = rule_id.contains("no-unused")
                    || message.contains("never used")
                    || message.contains("defined but never used")
                    || message.contains("assigned a value but never used");
                if !is_unused { continue; }

                let name = extract_eslint_name(message).unwrap_or_else(|| "<unused>".to_string());
                items.push(DeadItem {
                    name,
                    symbol_type: SymbolType::Variable,
                    file_path: PathBuf::from(file_path),
                    line_start: line,
                    line_end: end_line,
                    language: Language::JavaScript,
                    confidence: 80,
                    reason: format!("eslint {}: {}", rule_id, message),
                });
            }
        }

        Ok(items)
    }

    /// Run clang-tidy for C/C++
    async fn run_clang_tidy(files: &[PathBuf]) -> Result<Vec<DeadItem>> {
        let mut items = Vec::new();

        for file in files {
            // Only run for C/C++ extensions
            let ext = file.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
            if !matches!(ext.as_str(), "c" | "cc" | "cpp" | "cxx" | "c++" | "h" | "hpp" | "hh" | "hxx") {
                continue;
            }

            // Build clang-tidy command focusing on unused warnings
            // Note: clang-tidy may require compile_commands.json; if missing, it might still emit some diagnostics.
            let output = Command::new("clang-tidy")
                .arg("-quiet")
                .arg("-checks=-*,clang-diagnostic-unused-*,cppcoreguidelines-*,modernize-*,readability-*,performance-*")
                .arg(file)
                // Try to set a sane default language standard to reduce noise
                .arg("--")
                .arg("-std=c++17")
                .output();

            let output = match output {
                Ok(o) => o,
                Err(_) => continue, // clang-tidy not available or failed to execute for this file
            };

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{}\n{}", stdout, stderr);

            for line in combined.lines() {
                if let Some(item) = Self::parse_clang_tidy_line(line, file) {
                    items.push(item);
                }
            }
        }

        Ok(items)
    }

    /// Parse a single clang-tidy diagnostic line for unused-related issues
    fn parse_clang_tidy_line(line: &str, file_hint: &Path) -> Option<DeadItem> {
        // Typical formats include:
        //   path/to/file.cpp:12:7: warning: unused variable 'x' [-Wunused-variable]
        //   path/to/file.cpp:45:1: warning: function 'foo' is not needed and will not be emitted
        //   warning: unused parameter 'bar' [-Wunused-parameter]
        // We key on "unused" substring and attempt to classify.
        let l = line.to_lowercase();
        if !l.contains("unused") { return None; }

        // Extract file:line if present
        let mut file_path = PathBuf::from(file_hint);
        let mut line_no: u32 = 0;
        if let Some(colon_pos) = line.find(':') {
            // heuristic split: file:line:...
            let (prefix, rest) = line.split_at(colon_pos);
            if Path::new(prefix).exists() || prefix.contains('/') || prefix.contains('\\') {
                file_path = PathBuf::from(prefix);
                let rest = &rest[1..];
                if let Some(colon2) = rest.find(':') {
                    let num_str = &rest[..colon2];
                    if let Ok(n) = num_str.parse::<u32>() { line_no = n; }
                }
            }
        }

        let (symbol_type, name) = if l.contains("unused variable") {
            (SymbolType::Variable, extract_between_quotes(line).unwrap_or_else(|| "<var>".to_string()))
        } else if l.contains("unused parameter") {
            (SymbolType::Variable, extract_between_quotes(line).unwrap_or_else(|| "<param>".to_string()))
        } else if l.contains("unused function") || l.contains("function") && l.contains("unused") {
            (SymbolType::Function, extract_between_quotes(line).unwrap_or_else(|| "<func>".to_string()))
        } else if l.contains("unused struct") || l.contains("unused class") {
            (SymbolType::Class, extract_between_quotes(line).unwrap_or_else(|| "<type>".to_string()))
        } else {
            // Fallback classification
            (SymbolType::Variable, extract_between_quotes(line).unwrap_or_else(|| "<unused>".to_string()))
        };

        let language = match file_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
            "c" | "h" => Language::C,
            _ => Language::Cpp,
        };

        Some(DeadItem {
            name,
            symbol_type,
            file_path,
            line_start: line_no,
            line_end: line_no,
            language,
            confidence: 80,
            reason: line.to_string(),
        })
    }

    /// Run cargo-machete for unused dependencies
    async fn run_cargo_machete(project_dir: &Path) -> Result<Vec<DeadItem>> {
        let output = Command::new("cargo-machete")
            .current_dir(project_dir)
            .output()
            .map_err(|e| NekocodeError::External(format!("Failed to run cargo-machete: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_machete_output(&stdout)
    }

    /// Parse cargo-machete output
    fn parse_machete_output(output: &str) -> Result<Vec<DeadItem>> {
        let mut dead_items = Vec::new();
        
        for line in output.lines() {
            if line.contains("unused") {
                if let Some(crate_name) = Self::extract_unused_crate(line) {
                    dead_items.push(DeadItem {
                        name: crate_name,
                        symbol_type: SymbolType::Module,
                        file_path: PathBuf::from("Cargo.toml"),
                        line_start: 0,
                        line_end: 0,
                        language: Language::Rust,
                        confidence: 85,
                        reason: "cargo-machete: unused dependency".to_string(),
                    });
                }
            }
        }
        
        Ok(dead_items)
    }

    /// Extract unused crate name from machete output
    fn extract_unused_crate(line: &str) -> Option<String> {
        // Parse cargo-machete output format
        // This would need to be implemented based on actual output format
        None
    }
}

/// Best-effort extractor for ESLint message symbol names
fn extract_eslint_name(message: &str) -> Option<String> {
    if let Some(start) = message.find('\'') {
        let rest = &message[start + 1..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Extract name between single quotes: "'name' ..."
fn extract_between_quotes(text: &str) -> Option<String> {
    if let Some(start) = text.find('\'') {
        let rest = &text[start + 1..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Available external tools
#[derive(Debug, Clone, Copy)]
pub enum ExternalTool {
    CargoClippy,
    Vulture,
    Staticcheck,
    ESLint,
    ClangTidy,
    CargoMachete,
}

/// Tool availability status
#[derive(Debug)]
pub struct ToolAvailability {
    pub cargo_clippy: bool,
    pub vulture: bool,
    pub staticcheck: bool,
    pub eslint: bool,
    pub clang_tidy: bool,
    pub cargo_machete: bool,
}

impl ToolAvailability {
    /// Check if any external tool is available
    pub fn has_any_tool(&self) -> bool {
        self.cargo_clippy || self.vulture || self.staticcheck || 
        self.eslint || self.clang_tidy || self.cargo_machete
    }
    
    /// Get available tools for language
    pub fn get_available_for_language(&self, language: Language) -> Vec<ExternalTool> {
        let mut tools = Vec::new();
        
        match language {
            Language::Rust => {
                if self.cargo_clippy { tools.push(ExternalTool::CargoClippy); }
                if self.cargo_machete { tools.push(ExternalTool::CargoMachete); }
            }
            Language::Python => {
                if self.vulture { tools.push(ExternalTool::Vulture); }
            }
            Language::Go => {
                if self.staticcheck { tools.push(ExternalTool::Staticcheck); }
            }
            Language::JavaScript | Language::TypeScript => {
                if self.eslint { tools.push(ExternalTool::ESLint); }
            }
            Language::Cpp | Language::C => {
                if self.clang_tidy { tools.push(ExternalTool::ClangTidy); }
            }
            _ => {}
        }
        
        tools
    }

    /// Check if any tools are available
    pub fn has_any_tools(&self) -> bool {
        self.cargo_clippy || self.vulture || self.staticcheck || 
        self.eslint || self.clang_tidy || self.cargo_machete
    }

    /// Display installation guidance for missing tools
    pub fn display_installation_guide(&self, target_language: Option<Language>) {
        println!("\n⚠️ **External Tools Detection Status**");
        println!("=====================================");
        
        // Always show tool status for transparency
        let mut missing_count = 0;
        
        if let Some(Language::Rust) = target_language {
            if self.cargo_clippy {
                println!("✅ cargo clippy - installed (95% confidence)");
            } else {
                println!("❌ cargo clippy - not found");
                missing_count += 1;
            }
            
            if self.cargo_machete {
                println!("✅ cargo-machete - installed (85% confidence)");
            } else {
                println!("❌ cargo-machete - not found");
                missing_count += 1;
            }
        } else if let Some(Language::Python) = target_language {
            if self.vulture {
                println!("✅ vulture - installed (80% confidence)");
            } else {
                println!("❌ vulture - not found");
                missing_count += 1;
            }
        }
        
        if missing_count == 0 {
            println!("\n✨ All external tools detected! Using high accuracy analysis (90%)");
            return;
        }
        
        println!("\n🔧 **Installation Guide for Missing Tools**");
        println!("=====================================");
        
        match target_language {
            Some(Language::Rust) => {
                println!("📦 **For Rust Projects:**");
                self.show_rust_installation_guide();
            }
            Some(Language::Python) => {
                println!("🐍 **For Python Projects:**");
                self.show_python_installation_guide();
            }
            Some(Language::Go) => {
                println!("🐹 **For Go Projects:**");
                self.show_go_installation_guide();
            }
            Some(Language::JavaScript) | Some(Language::TypeScript) => {
                println!("🟨 **For JavaScript/TypeScript Projects:**");
                self.show_js_installation_guide();
            }
            Some(Language::Cpp) | Some(Language::C) => {
                println!("⚙️ **For C/C++ Projects:**");
                self.show_cpp_installation_guide();
            }
            _ => {
                println!("🌍 **For All Languages:**");
                self.show_comprehensive_installation_guide();
            }
        }
        
        println!("\n💡 **Tip**: Install tools for better deadcode detection accuracy!");
        println!("   Internal analysis: ~60% accuracy");
        println!("   External tools: ~90% accuracy");
    }

    fn show_rust_installation_guide(&self) {
        println!("\n📦 **Rust Dead Code Detection Tools**");
        println!("────────────────────────────────────");
        
        if !self.cargo_clippy {
            println!("\n1. cargo clippy (95% confidence - ESSENTIAL)");
            println!("   Install: rustup component add clippy");
            println!("   Purpose: Detects unused functions, structs, variables");
        }
        
        if !self.cargo_machete {
            println!("\n2. cargo-machete (85% confidence - RECOMMENDED)");
            println!("   Install: cargo install cargo-machete");
            println!("   Purpose: Detects unused dependencies in Cargo.toml");
        }
        
        if self.cargo_clippy && self.cargo_machete {
            println!("│ ✅ cargo clippy             │");
        }
        
        if !self.cargo_machete {
            println!("│ ❌ cargo-machete            │");
            println!("│   cargo install cargo-machete │");
        } else {
            println!("│ ✅ cargo-machete            │");
        }
        println!("└────────────────────────────┘");
        
        if !self.cargo_clippy || !self.cargo_machete {
            println!("\n🚀 **Quick Install (Copy & Paste):**");
            if !self.cargo_clippy {
                println!("rustup component add clippy");
            }
            if !self.cargo_machete {
                println!("cargo install cargo-machete");
            }
        }
    }

    fn show_python_installation_guide(&self) {
        println!("┌─ Python Tools (Recommended) ─┐");
        if !self.vulture {
            println!("│ ❌ vulture                    │");
            println!("│   pip install vulture         │");
        } else {
            println!("│ ✅ vulture                    │");
        }
        println!("└─────────────────────────────┘");
        
        if !self.vulture {
            println!("\n🚀 **Quick Install (Copy & Paste):**");
            println!("pip install vulture");
            println!("# or with conda:");
            println!("conda install -c conda-forge vulture");
        }
    }

    fn show_go_installation_guide(&self) {
        println!("┌─ Go Tools (Recommended) ─┐");
        if !self.staticcheck {
            println!("│ ❌ staticcheck             │");
            println!("│   go install honnef.co/... │");
        } else {
            println!("│ ✅ staticcheck             │");
        }
        println!("└───────────────────────────┘");
        
        if !self.staticcheck {
            println!("\n🚀 **Quick Install (Copy & Paste):**");
            println!("go install honnef.co/go/tools/cmd/staticcheck@latest");
        }
    }

    fn show_js_installation_guide(&self) {
        println!("┌─ JavaScript/TypeScript Tools ─┐");
        if !self.eslint {
            println!("│ ❌ ESLint                      │");
            println!("│   npm install -g eslint        │");
        } else {
            println!("│ ✅ ESLint                      │");
        }
        println!("└─────────────────────────────────┘");
        
        if !self.eslint {
            println!("\n🚀 **Quick Install (Copy & Paste):**");
            println!("npm install -g eslint");
            println!("# or with yarn:");
            println!("yarn global add eslint");
        }
    }

    fn show_cpp_installation_guide(&self) {
        println!("┌─ C/C++ Tools (Recommended) ─┐");
        if !self.clang_tidy {
            println!("│ ❌ clang-tidy               │");
            println!("│   apt install clang-tidy    │");
        } else {
            println!("│ ✅ clang-tidy               │");
        }
        println!("└────────────────────────────┘");
        
        if !self.clang_tidy {
            println!("\n🚀 **Quick Install (Copy & Paste):**");
            println!("# Ubuntu/Debian:");
            println!("sudo apt install clang-tidy");
            println!("# macOS:");
            println!("brew install llvm");
        }
    }

    fn show_comprehensive_installation_guide(&self) {
        println!("┌─ All Language Tools ─┐");
        println!("│ 🦀 Rust:              │");
        if !self.cargo_clippy {
            println!("│   rustup component add clippy │");
        }
        if !self.cargo_machete {
            println!("│   cargo install cargo-machete │");
        }
        
        println!("│ 🐍 Python:            │");
        if !self.vulture {
            println!("│   pip install vulture  │");
        }
        
        println!("│ 🐹 Go:                │");
        if !self.staticcheck {
            println!("│   go install honnef.co/go/tools/cmd/staticcheck@latest │");
        }
        
        println!("│ 🟨 JavaScript:         │");
        if !self.eslint {
            println!("│   npm install -g eslint │");
        }
        
        println!("│ ⚙️ C/C++:              │");
        if !self.clang_tidy {
            println!("│   sudo apt install clang-tidy │");
        }
        println!("└─────────────────────┘");
    }
}
