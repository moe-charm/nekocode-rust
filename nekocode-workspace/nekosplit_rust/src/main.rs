use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
use regex::Regex;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::collections::{HashSet, BTreeMap};
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "nekosplit_rust")] 
#[command(about = "Lightweight Rust code splitter (outline + minimal split)")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show an outline of top-level symbols in a Rust file
    Outline { file: PathBuf, #[arg(long, default_value_t = 200)] limit: usize },
    /// Split a function/struct/enum/trait/impl from a Rust file into a target file
    Split {
        file: PathBuf,
        /// Symbol name to extract (e.g., function name)
        #[arg(long)]
        symbol: String,
        /// Kind of symbol (fn/struct/enum/trait/impl)
        #[arg(long, value_enum, default_value_t = Kind::Fn)]
        kind: Kind,
        /// Target file path to write the symbol into
        #[arg(long)]
        to: PathBuf,
        /// Visibility to suggest for moved items (no automatic rewrite yet)
        #[arg(long, default_value = "pub(super)")]
        vis: String,
        /// Also update parent mod.rs (create if missing)
        #[arg(long)]
        update_mod: bool,
        /// Make generated mod public (with --update-mod only)
        #[arg(long)]
        public: bool,
        /// Copy top-level use statements from source file to target (dedup)
        #[arg(long, default_value_t = true)]
        copy_uses: bool,
        /// Actually apply changes (otherwise dry-run)
        #[arg(long)]
        apply: bool,
    },
    /// Suggest top-K large symbols to split
    Suggest {
        file: PathBuf,
        /// Top-K candidates
        #[arg(long, default_value_t = 5)]
        top: usize,
        /// Minimum lines threshold
        #[arg(long, default_value_t = 80)]
        min_loc: usize,
        /// Mapping hints (e.g. "execute:execution/*,op:ops/*,model:models/*")
        #[arg(long)]
        map: Option<String>,
        /// Output JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Apply suggested splits (batch)
    ApplySuggest {
        file: PathBuf,
        /// Max number of items to apply
        #[arg(long, default_value_t = 1)]
        max_steps: usize,
        /// Minimum lines threshold
        #[arg(long, default_value_t = 80)]
        min_loc: usize,
        /// Mapping hints (e.g. "execute:execution/*,op:ops/*,model:models/*")
        #[arg(long)]
        map: Option<String>,
        /// Update mod.rs declarations
        #[arg(long)]
        update_mod: bool,
        /// Make module public (with --update-mod)
        #[arg(long)]
        public: bool,
        /// Copy top-level uses to targets
        #[arg(long, default_value_t = true)]
        copy_uses: bool,
        /// Run cargo check after each step and rollback on failure
        #[arg(long, default_value_t = false)]
        check: bool,
        /// Dry-run only
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum, Eq, PartialEq, Hash)]
enum Kind { Fn, Struct, Enum, Trait, Impl }

fn read_to_string(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}

fn print_outline(src: &str, limit: usize) {
    // Simple regex-based outline (no dependencies download required)
    let re = Regex::new(r"(?m)^(?:\s*#\[[^\]]*\]\s*)*(pub\s+)?(struct|enum|trait|fn)\s+([A-Za-z0-9_]+)").unwrap();
    let re_impl = Regex::new(r"(?m)^(?:\s*#\[[^\]]*\]\s*)*(pub\s+)?impl(?:\s*<[^>]+>\s*)?\s*([A-Za-z0-9_]+)").unwrap();
    println!("🗂️ Outline (showing up to {} entries):", limit);
    for (i, cap) in re.captures_iter(src).take(limit).enumerate() {
        let vis = cap.get(1).map(|m| m.as_str()).unwrap_or("").trim();
        let kind = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let name = cap.get(3).map(|m| m.as_str()).unwrap_or("");
        println!("{:>3}. {:<5} {:<10} {}", i + 1, vis, kind, name);
    }
    for (i, cap) in re_impl.captures_iter(src).take(limit).enumerate() {
        let vis = cap.get(1).map(|m| m.as_str()).unwrap_or("").trim();
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        println!("     {:<5} {:<10} {}", vis, "impl", name);
    }
}

// Minimal function block extractor: finds `fn name` and captures until matching closing brace using a simple
// bracket counter. This is best-effort and intended for MVP/dry-run.
fn extract_fn_block(src: &str, name: &str) -> Option<(usize, usize)> {
    let pattern = format!("fn {}", name);
    let start = src.find(&pattern)?;
    // Find the opening brace after the signature
    let sig_end = src[start..].find('{')? + start;
    let mut depth = 0i32;
    let mut end_idx = sig_end;
    for (i, ch) in src[sig_end..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end_idx = sig_end + i + 1; // include closing brace
                    break;
                }
            }
            _ => {}
        }
    }
    if end_idx <= sig_end { return None; }
    // expand backward to line start
    let line_start = src[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    Some((line_start, end_idx))
}

fn extract_block_with_braces(src: &str, header_start: usize) -> Option<usize> {
    // find first '{' from header_start
    let brace = src[header_start..].find('{')? + header_start;
    let mut depth = 0i32;
    for (i, ch) in src[brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(brace + i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_header_start(src: &str, kind: Kind, name: &str) -> Option<usize> {
    match kind {
        Kind::Fn => src.find(&format!("fn {}", name)),
        Kind::Struct => src.find(&format!("struct {}", name)),
        Kind::Enum => src.find(&format!("enum {}", name)),
        Kind::Trait => src.find(&format!("trait {}", name)),
        Kind::Impl => {
            // Try a few patterns
            let pat = format!("impl {}", name);
            src.find(&pat)
        }
    }
}

fn extract_item(src: &str, kind: Kind, name: &str) -> Option<(usize, usize)> {
    let start = find_header_start(src, kind, name)?;
    // extend start to line start (to capture attributes/visibility)
    let line_start = src[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    match kind {
        Kind::Fn | Kind::Enum | Kind::Trait | Kind::Impl => {
            let end = extract_block_with_braces(src, start)?;
            Some((line_start, end))
        }
        Kind::Struct => {
            // could be tuple/semicolon or braces
            if let Some(end) = extract_block_with_braces(src, start) {
                Some((line_start, end))
            } else {
                // find next semicolon
                let semi = src[start..].find(';')? + start + 1;
                Some((line_start, semi))
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    kind: Kind,
    name: String,
    start: usize,
    end: usize,
    lines: usize,
}

fn snake_name(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 { out.push('_'); }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn simple_candidates(src: &str) -> Vec<Candidate> {
    let mut items: Vec<(Kind, String, usize)> = Vec::new();
    let re = Regex::new(r"(?m)^(?:\s*#\[[^\]]*\]\s*)*(?:pub\s+)?(struct|enum|trait|fn)\s+([A-Za-z0-9_]+)").unwrap();
    for cap in re.captures_iter(src) {
        let kind = match cap.get(1).unwrap().as_str() { "struct" => Kind::Struct, "enum" => Kind::Enum, "trait" => Kind::Trait, _ => Kind::Fn };
        let name = cap.get(2).unwrap().as_str().to_string();
        let start = cap.get(0).unwrap().start();
        items.push((kind, name, start));
    }
    // impl Name
    let re_impl = Regex::new(r"(?m)^(?:\s*#\[[^\]]*\]\s*)*(?:pub\s+)?impl(?:\s*<[^>]+>\s*)?\s*([A-Za-z0-9_]+)").unwrap();
    for cap in re_impl.captures_iter(src) {
        let name = cap.get(1).unwrap().as_str().to_string();
        let start = cap.get(0).unwrap().start();
        items.push((Kind::Impl, name, start));
    }
    let mut set: HashSet<(Kind, String, usize)> = HashSet::new();
    let mut out = Vec::new();
    for (k, n, s) in items {
        if !set.insert((k, n.clone(), s)) { continue; }
        if let Some((ls, le)) = extract_item(src, k, &n) {
            let text = &src[ls..le];
            let lines = text.lines().count();
            out.push(Candidate { kind: k, name: n, start: ls, end: le, lines });
        }
    }
    out
}

fn parse_map(map: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for pair in map.split(',') {
        if let Some((k, v)) = pair.split_once(':') {
            m.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    m
}

fn suggest_target(file: &Path, cand: &Candidate, map_str: Option<&str>) -> PathBuf {
    let parent = file.parent().unwrap_or(Path::new("."));
    let mut subdir = String::new();
    let lower = cand.name.to_lowercase();
    if let Some(ms) = map_str {
        let m = parse_map(ms);
        for (k, v) in m.iter() {
            if lower.contains(&k.to_lowercase()) { subdir = v.clone(); break; }
        }
    }
    if subdir.is_empty() {
        if lower.contains("op") || lower.contains("operator") { subdir = "ops/*".into(); }
        else if lower.contains("exec") { subdir = "execution/*".into(); }
        else if lower.contains("model") || lower.contains("class") { subdir = "models/*".into(); }
        else if cand.kind == Kind::Trait { subdir = "traits/*".into(); }
        else { subdir = "*".into(); }
    }
    let file_name = format!("{}.rs", snake_name(&cand.name));
    let replaced = subdir.replace('*', &file_name);
    let rel = Path::new(&replaced);
    parent.join(rel)
}

fn cargo_check(project_root: &Path) -> bool {
    if !project_root.join("Cargo.toml").exists() { return true; }
    let status = Command::new("cargo").arg("check").current_dir(project_root).output();
    match status { Ok(o) => o.status.success(), Err(_) => true }
}

fn collect_top_level_uses(src: &str) -> Vec<String> {
    let re = Regex::new(r"(?m)^\s*use\s+[^;]+;\s*$").unwrap();
    re.captures_iter(src)
        .filter_map(|c| c.get(0).map(|m| m.as_str().to_string()))
        .collect()
}

fn prepend_uses_if_missing(target_path: &Path, uses: &[String]) -> Result<()> {
    let mut existing = String::new();
    if target_path.exists() {
        existing = read_to_string(target_path)?;
    }
    let mut to_add: Vec<&String> = vec![];
    for u in uses {
        if !existing.contains(u) {
            to_add.push(u);
        }
    }
    if to_add.is_empty() { return Ok(()); }
    ensure_parent_dir(target_path)?;
    let mut out = String::new();
    // place uses at top
    for u in &to_add { out.push_str(u); out.push('\n'); }
    out.push_str(&existing);
    fs::write(target_path, out)?;
    Ok(())
}

fn update_mod_rs(target_file: &Path, public: bool) -> Result<()> {
    if let Some(parent) = target_file.parent() {
        let module_name = target_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid target file name"))?;
        let mod_rs = parent.join("mod.rs");
        let decl = if public { format!("pub mod {};", module_name) } else { format!("mod {};", module_name) };
        if mod_rs.exists() {
            let content = read_to_string(&mod_rs)?;
            if !content.contains(&decl) {
                let mut f = fs::OpenOptions::new().append(true).open(&mod_rs)?;
                writeln!(f, "{}", decl)?;
            }
        } else {
            let mut f = fs::File::create(&mod_rs)?;
            writeln!(f, "{}", decl)?;
        }
    }
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    match cli.cmd {
        Commands::Outline { file, limit } => {
            let src = read_to_string(&file)?;
            print_outline(&src, limit);
        }
        Commands::Split { file, symbol, kind, to, vis, update_mod, public, copy_uses, apply } => {
            let src = read_to_string(&file)?;
            if let Some((s, e)) = extract_item(&src, kind, &symbol) {
                let block = &src[s..e];
                println!("🔎 Found {:?} '{}': bytes [{}..{}] (~{} chars)", kind, symbol, s, e, block.len());
                println!("   Suggested visibility: {} (no automatic rewrite in MVP)", vis);

                if apply {
                    // Optionally copy top-level uses before appending
                    if copy_uses {
                        let uses = collect_top_level_uses(&src);
                        if !uses.is_empty() { prepend_uses_if_missing(&to, &uses)?; }
                    }
                    // Append extracted block
                    {
                        ensure_parent_dir(&to)?;
                        let mut out = fs::OpenOptions::new().create(true).append(true).open(&to)?;
                        writeln!(out, "\n// ---- extracted by nekosplit_rust ----")?;
                        out.write_all(block.as_bytes())?;
                    }

                    // Remove from source by replacing range with comment marker
                    let mut new_src = String::with_capacity(src.len());
                    new_src.push_str(&src[..s]);
                    new_src.push_str(&format!("// [nekosplit] moved {:?} {} -> {}\n", kind, symbol, to.display()));
                    new_src.push_str(&src[e..]);
                    fs::write(&file, new_src)?;
                    if update_mod { update_mod_rs(&to, public)?; }
                    println!("✅ Applied: moved {:?} '{}' to {}", kind, symbol, to.display());
                    if !update_mod { println!("ℹ️ Tip: use --update-mod to add module declaration automatically"); }
                } else {
                    println!("💡 Dry-run. Would append to: {}", to.display());
                    println!("   And mark removal in source: {}", file.display());
                    println!("   (Run with --apply to perform changes)");
                }
            } else {
                return Err(anyhow!("Symbol '{:?} {}' not found or block matching failed", kind, symbol));
            }
        }
        Commands::Suggest { file, top, min_loc, map, json } => {
            let src = read_to_string(&file)?;
            let mut cands = simple_candidates(&src);
            cands.sort_by_key(|c| usize::MAX - c.lines);
            cands.retain(|c| c.lines >= min_loc);
            let mut out_rows = Vec::new();
            for c in cands.iter().take(top) {
                let to = suggest_target(&file, c, map.as_deref());
                out_rows.push((format!("{:?}", c.kind), c.name.clone(), c.lines, to.display().to_string()));
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&out_rows)?);
            } else {
                println!("📋 Suggest top {} (min_loc={}):", top, min_loc);
                for (i, (k, n, l, t)) in out_rows.iter().enumerate() {
                    println!("{:>2}. {:<6} {:<32} {:>5} loc  → {}", i + 1, k, n, l, t);
                }
            }
        }
        Commands::ApplySuggest { file, max_steps, min_loc, map, update_mod, public, copy_uses, check, dry_run } => {
            let src = read_to_string(&file)?;
            let mut cands = simple_candidates(&src);
            cands.sort_by_key(|c| usize::MAX - c.lines);
            cands.retain(|c| c.lines >= min_loc);
            let project_root = file.ancestors().find(|p| p.join("Cargo.toml").exists()).map(|p| p.to_path_buf());
            let mut applied = 0usize;
            for cand in cands.into_iter().take(max_steps) {
                let to = suggest_target(&file, &cand, map.as_deref());
                println!("➡️  Split {:?} '{}' ({} loc) → {}", cand.kind, cand.name, cand.lines, to.display());
                if dry_run { continue; }
                let orig_src = read_to_string(&file)?;
                let orig_to = if to.exists() { Some(read_to_string(&to)?) } else { None };
                if let Some((s,e)) = extract_item(&orig_src, cand.kind, &cand.name) {
                    // copy uses
                    if copy_uses {
                        let uses = collect_top_level_uses(&orig_src);
                        if !uses.is_empty() { let _ = prepend_uses_if_missing(&to, &uses); }
                    }
                    // append block
                    ensure_parent_dir(&to)?;
                    {
                        let mut out = fs::OpenOptions::new().create(true).append(true).open(&to)?;
                        writeln!(out, "\n// ---- extracted by nekosplit_rust (apply-suggest) ----")?;
                        out.write_all(orig_src[s..e].as_bytes())?;
                    }
                    // source marker
                    let mut new_src = String::new();
                    new_src.push_str(&orig_src[..s]);
                    new_src.push_str(&format!("// [nekosplit] moved {:?} {} -> {}\n", cand.kind, cand.name, to.display()));
                    new_src.push_str(&orig_src[e..]);
                    fs::write(&file, new_src.clone())?;
                    if update_mod { let _ = update_mod_rs(&to, public); }
                    if check {
                        if let Some(root) = project_root.as_ref() {
                            if !cargo_check(root) {
                                // rollback
                                fs::write(&file, orig_src)?;
                                if let Some(prev) = orig_to { fs::write(&to, prev)?; } else { let _ = fs::remove_file(&to); }
                                println!("⛔ Failed cargo check. Rolled back this step.");
                                continue;
                            }
                        }
                    }
                    applied += 1;
                }
            }
            println!("✅ Applied {} step(s)", applied);
        }
    }
    Ok(())
}
