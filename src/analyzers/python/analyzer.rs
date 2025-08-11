//! Python analyzer implementation using pest parser
//! 
//! This module provides Python analysis capabilities,
//! detecting classes, functions, methods, imports, decorators, and complexity metrics.

use anyhow::Result;
use async_trait::async_trait;
use pest::Parser;
use pest_derive::Parser;

use crate::core::types::{
    AnalysisResult, ClassInfo, ComplexityInfo, FileInfo, FunctionInfo, ImportInfo, 
    ImportType, Language, ExportInfo, ExportType, FunctionCall
};
use crate::analyzers::traits::LanguageAnalyzer;

#[derive(Parser)]
#[grammar = "analyzers/python/grammar.pest"]
pub struct PythonParser;

/// Python analyzer
pub struct PythonAnalyzer {
    // Internal state for parsing
}

impl PythonAnalyzer {
    pub fn new() -> Self {
        Self {}
    }
    
    /// Extract functions from parsed content
    fn extract_functions(&self, pairs: pest::iterators::Pairs<Rule>) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        
        for pair in pairs {
            match pair.as_rule() {
                Rule::function_def => {
                    if let Some(func) = self.parse_function_definition(pair) {
                        functions.push(func);
                    }
                }
                Rule::lambda_func => {
                    if let Some(func) = self.parse_lambda_function(pair) {
                        functions.push(func);
                    }
                }
                _ => {
                    // Recursively process inner pairs
                    functions.extend(self.extract_functions(pair.into_inner()));
                }
            }
        }
        
        functions
    }
    
    /// Parse a function definition
    fn parse_function_definition(&self, pair: pest::iterators::Pair<Rule>) -> Option<FunctionInfo> {
        let mut function_name = String::new();
        let mut is_async = false;
        let mut parameters = Vec::new();
        let mut decorators = Vec::new();
        
        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::decorator => {
                    decorators.push(inner_pair.as_str().to_string());
                }
                Rule::async_keyword => {
                    is_async = true;
                }
                Rule::identifier => {
                    if function_name.is_empty() {
                        function_name = inner_pair.as_str().to_string();
                    }
                }
                Rule::parameter_list => {
                    parameters = self.parse_parameters(inner_pair);
                }
                _ => {}
            }
        }
        
        if !function_name.is_empty() {
            let mut func = FunctionInfo::new(function_name);
            func.is_async = is_async;
            func.parameters = parameters;
            
            // Add decorator information to metadata
            if !decorators.is_empty() {
                func.metadata.insert("decorators".to_string(), decorators.join(", "));
            }
            
            Some(func)
        } else {
            None
        }
    }
    
    /// Parse a lambda function
    fn parse_lambda_function(&self, pair: pest::iterators::Pair<Rule>) -> Option<FunctionInfo> {
        let mut parameters = Vec::new();
        
        for inner_pair in pair.into_inner() {
            if inner_pair.as_rule() == Rule::parameter_list {
                parameters = self.parse_parameters(inner_pair);
            }
        }
        
        let mut func = FunctionInfo::new("lambda".to_string());
        func.parameters = parameters;
        func.metadata.insert("is_lambda".to_string(), "true".to_string());
        Some(func)
    }
    
    /// Parse function parameters
    fn parse_parameters(&self, pair: pest::iterators::Pair<Rule>) -> Vec<String> {
        let mut parameters = Vec::new();
        
        for inner_pair in pair.into_inner() {
            if inner_pair.as_rule() == Rule::parameter {
                for param_pair in inner_pair.into_inner() {
                    if param_pair.as_rule() == Rule::identifier {
                        parameters.push(param_pair.as_str().to_string());
                        break; // Take first identifier as parameter name
                    }
                }
            }
        }
        
        parameters
    }
    
    /// Extract classes from parsed content
    fn extract_classes(&self, pairs: pest::iterators::Pairs<Rule>) -> Vec<ClassInfo> {
        let mut classes = Vec::new();
        
        for pair in pairs {
            match pair.as_rule() {
                Rule::class_def => {
                    if let Some(class) = self.parse_class_definition(pair) {
                        classes.push(class);
                    }
                }
                _ => {
                    // Recursively process inner pairs
                    classes.extend(self.extract_classes(pair.into_inner()));
                }
            }
        }
        
        classes
    }
    
    /// Parse a class definition
    fn parse_class_definition(&self, pair: pest::iterators::Pair<Rule>) -> Option<ClassInfo> {
        let mut class_name = String::new();
        let mut parent_classes = Vec::new();
        let mut decorators = Vec::new();
        
        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::decorator => {
                    decorators.push(inner_pair.as_str().to_string());
                }
                Rule::identifier => {
                    if class_name.is_empty() {
                        class_name = inner_pair.as_str().to_string();
                    }
                }
                Rule::inheritance => {
                    parent_classes = self.parse_inheritance(inner_pair);
                }
                _ => {}
            }
        }
        
        if !class_name.is_empty() {
            let mut class = ClassInfo::new(class_name);
            if !parent_classes.is_empty() {
                class.parent_class = Some(parent_classes[0].clone());
                if parent_classes.len() > 1 {
                    class.metadata.insert("multiple_inheritance".to_string(), parent_classes.join(", "));
                }
            }
            
            // Add decorator information to metadata
            if !decorators.is_empty() {
                class.metadata.insert("decorators".to_string(), decorators.join(", "));
            }
            
            // Extract methods would require additional parsing
            // For now, we'll use regex fallback in the main analyze function
            
            Some(class)
        } else {
            None
        }
    }
    
    /// Parse inheritance list
    fn parse_inheritance(&self, pair: pest::iterators::Pair<Rule>) -> Vec<String> {
        let mut parent_classes = Vec::new();
        
        for inner_pair in pair.into_inner() {
            if inner_pair.as_rule() == Rule::class_list {
                for class_pair in inner_pair.into_inner() {
                    if class_pair.as_rule() == Rule::identifier {
                        parent_classes.push(class_pair.as_str().to_string());
                    }
                }
            }
        }
        
        parent_classes
    }
    
    /// Extract imports from parsed content
    fn extract_imports(&self, pairs: pest::iterators::Pairs<Rule>) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        
        for pair in pairs {
            match pair.as_rule() {
                Rule::from_import => {
                    if let Some(import) = self.parse_from_import(pair) {
                        imports.push(import);
                    }
                }
                Rule::simple_import => {
                    if let Some(import) = self.parse_simple_import(pair) {
                        imports.push(import);
                    }
                }
                _ => {
                    // Recursively process inner pairs
                    imports.extend(self.extract_imports(pair.into_inner()));
                }
            }
        }
        
        imports
    }
    
    /// Parse a from import statement
    fn parse_from_import(&self, pair: pest::iterators::Pair<Rule>) -> Option<ImportInfo> {
        let mut module_path = String::new();
        let mut imported_names = Vec::new();
        
        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::module_path => {
                    module_path = inner_pair.as_str().to_string();
                }
                Rule::import_list => {
                    for list_pair in inner_pair.into_inner() {
                        if list_pair.as_rule() == Rule::identifier {
                            imported_names.push(list_pair.as_str().to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        
        if !module_path.is_empty() {
            let mut import = ImportInfo::new(ImportType::PythonFromImport, module_path);
            import.imported_names = imported_names;
            Some(import)
        } else {
            None
        }
    }
    
    /// Parse a simple import statement
    fn parse_simple_import(&self, pair: pest::iterators::Pair<Rule>) -> Option<ImportInfo> {
        let mut module_path = String::new();
        let mut alias = None;
        
        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::module_path => {
                    module_path = inner_pair.as_str().to_string();
                }
                Rule::identifier => {
                    // This is the alias after 'as'
                    alias = Some(inner_pair.as_str().to_string());
                }
                _ => {}
            }
        }
        
        if !module_path.is_empty() {
            let mut import = ImportInfo::new(ImportType::PythonImport, module_path);
            if let Some(alias) = alias {
                import.metadata.insert("alias".to_string(), alias);
            }
            Some(import)
        } else {
            None
        }
    }
    
    /// Extract function calls for call analysis
    fn extract_function_calls(&self, pairs: pest::iterators::Pairs<Rule>) -> Vec<FunctionCall> {
        let mut calls = Vec::new();
        
        for pair in pairs {
            match pair.as_rule() {
                Rule::function_call => {
                    if let Some(call) = self.parse_function_call(pair, false) {
                        calls.push(call);
                    }
                }
                Rule::method_call => {
                    if let Some(call) = self.parse_function_call(pair, true) {
                        calls.push(call);
                    }
                }
                _ => {
                    // Recursively process inner pairs
                    calls.extend(self.extract_function_calls(pair.into_inner()));
                }
            }
        }
        
        calls
    }
    
    /// Parse a function call
    fn parse_function_call(&self, pair: pest::iterators::Pair<Rule>, is_method: bool) -> Option<FunctionCall> {
        let mut function_name = String::new();
        let mut object_name = None;
        let mut identifiers = Vec::new();
        
        for inner_pair in pair.into_inner() {
            if inner_pair.as_rule() == Rule::identifier {
                identifiers.push(inner_pair.as_str().to_string());
            }
        }
        
        if !identifiers.is_empty() {
            if is_method && identifiers.len() >= 2 {
                object_name = Some(identifiers[0].clone());
                function_name = identifiers[1].clone();
            } else {
                function_name = identifiers[0].clone();
            }
            
            let mut call = FunctionCall::new(function_name, 0); // Line number would need position tracking
            call.object_name = object_name;
            call.is_method_call = is_method;
            Some(call)
        } else {
            None
        }
    }
    
    /// Calculate complexity metrics for Python
    fn calculate_complexity(&self, content: &str) -> ComplexityInfo {
        let mut complexity = ComplexityInfo::new();
        
        // Count complexity-increasing constructs specific to Python
        let complexity_keywords = [
            "if ", "elif ", "else:", "for ", "while ", "try:", "except", 
            "finally:", "with ", "and ", "or ", "lambda", "yield",
            "async def", "await ", "list comprehension", "dict comprehension"
        ];
        
        for keyword in &complexity_keywords {
            let count = content.matches(keyword).count() as u32;
            complexity.cyclomatic_complexity += count;
        }
        
        // Count list/dict comprehensions which add complexity
        let comprehension_count = content.matches("[").count() + content.matches("{").count();
        complexity.cyclomatic_complexity += (comprehension_count as u32) / 4; // Rough approximation
        
        // Calculate nesting depth using indentation
        let mut current_depth = 0;
        let mut max_depth = 0;
        
        for line in content.lines() {
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            
            let leading_spaces = line.len() - line.trim_start().len();
            let indent_level = leading_spaces / 4; // Assuming 4-space indentation
            
            current_depth = indent_level as u32;
            max_depth = max_depth.max(current_depth);
        }
        
        complexity.max_nesting_depth = max_depth;
        complexity.update_rating();
        
        complexity
    }
    
    /// Update line numbers for Python constructs
    fn update_line_numbers(&self, content: &str, result: &mut AnalysisResult) {
        let lines: Vec<&str> = content.lines().collect();
        
        // Update function line numbers
        for function in &mut result.functions {
            if let Some(line_num) = self.find_line_number(&lines, &function.name, "def") {
                function.start_line = line_num;
                function.end_line = self.find_python_block_end_line(&lines, line_num);
            }
        }
        
        // Update class line numbers
        for class in &mut result.classes {
            if let Some(line_num) = self.find_line_number(&lines, &class.name, "class") {
                class.start_line = line_num;
                class.end_line = self.find_python_block_end_line(&lines, line_num);
            }
            
            // Extract and update methods for this class
            class.methods = self.extract_class_methods_python(content, &class.name, class.start_line, class.end_line);
        }
    }
    
    /// Find line number for a Python construct
    fn find_line_number(&self, lines: &[&str], name: &str, construct_type: &str) -> Option<u32> {
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains(construct_type) && trimmed.contains(name) && !trimmed.starts_with('#') {
                return Some((i + 1) as u32);
            }
        }
        None
    }
    
    /// Find Python block end line using indentation
    fn find_python_block_end_line(&self, lines: &[&str], start_line: u32) -> u32 {
        let start_idx = (start_line.saturating_sub(1)) as usize;
        
        if start_idx >= lines.len() {
            return start_line;
        }
        
        let start_line_content = lines[start_idx];
        let base_indent = start_line_content.len() - start_line_content.trim_start().len();
        
        // Look for the end of the block (when indentation returns to base level or less)
        for i in (start_idx + 1)..lines.len() {
            let line = lines[i];
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue; // Skip empty lines and comments
            }
            
            let line_indent = line.len() - line.trim_start().len();
            if line_indent <= base_indent {
                return i as u32;
            }
        }
        
        // If no end found, return file end
        lines.len() as u32
    }
    
    /// Extract class methods using Python-specific parsing
    fn extract_class_methods_python(&self, content: &str, class_name: &str, class_start: u32, class_end: u32) -> Vec<FunctionInfo> {
        let mut methods = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        let start_idx = (class_start.saturating_sub(1)) as usize;
        let end_idx = (class_end as usize).min(lines.len());
        
        for i in start_idx..end_idx {
            let line = lines[i];
            let trimmed = line.trim();
            
            // Look for method definitions (def keyword with proper indentation)
            if trimmed.starts_with("def ") || trimmed.contains("async def ") {
                if let Some(method_name) = self.extract_method_name_from_line(trimmed) {
                    let mut method = FunctionInfo::new(method_name);
                    method.start_line = (i + 1) as u32;
                    method.end_line = self.find_python_block_end_line(&lines, method.start_line);
                    method.is_async = trimmed.contains("async def");
                    method.metadata.insert("is_class_method".to_string(), "true".to_string());
                    
                    // Check for special methods
                    if method.name.starts_with("__") && method.name.ends_with("__") {
                        method.metadata.insert("is_dunder_method".to_string(), "true".to_string());
                    }
                    
                    methods.push(method);
                }
            }
        }
        
        methods
    }
    
    /// Extract method name from a line
    fn extract_method_name_from_line(&self, line: &str) -> Option<String> {
        // Remove async keyword if present
        let line = line.replace("async def", "def");
        
        if let Some(def_pos) = line.find("def ") {
            let after_def = &line[(def_pos + 4)..];
            if let Some(paren_pos) = after_def.find('(') {
                let method_name = after_def[..paren_pos].trim();
                if !method_name.is_empty() {
                    return Some(method_name.to_string());
                }
            }
        }
        None
    }
}

#[async_trait]
impl LanguageAnalyzer for PythonAnalyzer {
    fn get_language(&self) -> Language {
        Language::Python
    }
    
    fn get_language_name(&self) -> &'static str {
        "Python"
    }
    
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec![".py", ".pyw", ".pyi"]
    }
    
    async fn analyze(&mut self, content: &str, filename: &str) -> Result<AnalysisResult> {
        // Create file info
        let file_path = std::path::PathBuf::from(filename);
        let mut file_info = FileInfo::new(file_path);
        file_info.total_lines = content.lines().count() as u32;
        
        // Calculate basic line statistics
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                file_info.empty_lines += 1;
            } else if trimmed.starts_with("#") {
                file_info.comment_lines += 1;
            } else {
                file_info.code_lines += 1;
            }
        }
        
        file_info.code_ratio = if file_info.total_lines > 0 {
            file_info.code_lines as f64 / file_info.total_lines as f64
        } else {
            0.0
        };
        
        // Create analysis result
        let mut result = AnalysisResult::new(file_info, Language::Python);
        
        // Parse the Python content
        let mut parsing_succeeded = false;
        
        match PythonParser::parse(Rule::program, content) {
            Ok(mut pairs) => {
                if let Some(program) = pairs.next() {
                    let inner_pairs = program.into_inner();
                    
                    // Extract all constructs
                    let extracted_functions = self.extract_functions(inner_pairs.clone());
                    let extracted_classes = self.extract_classes(inner_pairs.clone());
                    let extracted_imports = self.extract_imports(inner_pairs.clone());
                    let extracted_calls = self.extract_function_calls(inner_pairs);
                    
                    // Only use pest results if they found meaningful content
                    if !extracted_functions.is_empty() || !extracted_classes.is_empty() || !extracted_imports.is_empty() {
                        result.functions = extracted_functions;
                        result.classes = extracted_classes;
                        result.imports = extracted_imports;
                        result.function_calls = extracted_calls;
                        parsing_succeeded = true;
                    }
                }
            }
            Err(e) => {
                // If parsing fails, log error but continue with fallback
                eprintln!("Warning: Pest parsing failed for {}: {}", filename, e);
            }
        }
        
        // Use regex fallback if pest parsing didn't succeed or found nothing significant
        if !parsing_succeeded {
            if result.functions.is_empty() {
                result.functions = self.regex_fallback_functions(content);
            }
            if result.classes.is_empty() {
                result.classes = self.regex_fallback_classes(content);
                
                // Extract methods for each class
                for class in &mut result.classes {
                    class.methods = self.extract_class_methods_python(content, &class.name, class.start_line, class.end_line);
                }
            }
            if result.imports.is_empty() {
                result.imports = self.regex_fallback_imports(content);
            }
        }
        
        // Update line numbers
        self.update_line_numbers(content, &mut result);
        
        // Build AST from analysis results (following C++ pattern)
        if !result.functions.is_empty() || !result.classes.is_empty() {
            let ast_root = self.build_ast_from_analysis(&result.functions, &result.classes, content);
            let mut ast_stats = crate::core::ast::ASTStatistics::default();
            ast_stats.update_from_root(&ast_root);
            result.ast_root = Some(ast_root);
            result.ast_statistics = Some(ast_stats);
        }
        
        // Calculate complexity
        result.complexity = self.calculate_complexity(content);
        
        // Build call frequency map
        for call in &result.function_calls {
            let entry = result.call_frequency.entry(call.full_name()).or_insert(0);
            *entry += 1;
        }
        
        // Update statistics
        result.update_statistics();
        
        Ok(result)
    }
}

impl PythonAnalyzer {
    /// Fallback function extraction using regex (when pest parsing fails)
    fn regex_fallback_functions(&self, content: &str) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        
        // Pattern 1: Regular function definitions
        if let Ok(re) = regex::Regex::new(r"(?m)^\s*(?:async\s+)?def\s+(\w+)\s*\(") {
            for caps in re.captures_iter(content) {
                if let Some(name) = caps.get(1) {
                    let mut func = FunctionInfo::new(name.as_str().to_string());
                    func.is_async = caps.get(0).unwrap().as_str().contains("async");
                    functions.push(func);
                }
            }
        }
        
        // Pattern 2: Lambda functions assigned to variables
        if let Ok(re) = regex::Regex::new(r"(?m)^\s*(\w+)\s*=\s*lambda") {
            for caps in re.captures_iter(content) {
                if let Some(name) = caps.get(1) {
                    let mut func = FunctionInfo::new(name.as_str().to_string());
                    func.metadata.insert("is_lambda".to_string(), "true".to_string());
                    functions.push(func);
                }
            }
        }
        
        functions
    }
    
    /// Fallback class extraction using regex (when pest parsing fails)
    fn regex_fallback_classes(&self, content: &str) -> Vec<ClassInfo> {
        let mut classes = Vec::new();
        
        // Pattern: Class definitions
        if let Ok(re) = regex::Regex::new(r"(?m)^\s*class\s+(\w+)(?:\s*\(\s*([^)]+)\s*\))?\s*:") {
            for caps in re.captures_iter(content) {
                if let Some(name) = caps.get(1) {
                    let mut class = ClassInfo::new(name.as_str().to_string());
                    
                    if let Some(parents) = caps.get(2) {
                        let parent_list: Vec<&str> = parents.as_str().split(',').map(|s| s.trim()).collect();
                        if !parent_list.is_empty() {
                            class.parent_class = Some(parent_list[0].to_string());
                            if parent_list.len() > 1 {
                                class.metadata.insert("multiple_inheritance".to_string(), parent_list.join(", "));
                            }
                        }
                    }
                    
                    classes.push(class);
                }
            }
        }
        
        classes
    }
    
    /// Fallback import extraction using regex (when pest parsing fails)
    fn regex_fallback_imports(&self, content: &str) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        
        // Pattern 1: from X import Y statements
        if let Ok(re) = regex::Regex::new(r"(?m)^\s*from\s+([a-zA-Z_][a-zA-Z0-9_.]*)\s+import\s+(.+)") {
            for caps in re.captures_iter(content) {
                if let Some(module) = caps.get(1) {
                    let mut import = ImportInfo::new(ImportType::PythonFromImport, module.as_str().to_string());
                    
                    if let Some(imports_str) = caps.get(2) {
                        let imported_names: Vec<String> = imports_str.as_str()
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        import.imported_names = imported_names;
                    }
                    
                    imports.push(import);
                }
            }
        }
        
        // Pattern 2: import X statements
        if let Ok(re) = regex::Regex::new(r"(?m)^\s*import\s+([a-zA-Z_][a-zA-Z0-9_.]*)(?:\s+as\s+(\w+))?") {
            for caps in re.captures_iter(content) {
                if let Some(module) = caps.get(1) {
                    let mut import = ImportInfo::new(ImportType::PythonImport, module.as_str().to_string());
                    
                    if let Some(alias) = caps.get(2) {
                        import.metadata.insert("alias".to_string(), alias.as_str().to_string());
                    }
                    
                    imports.push(import);
                }
            }
        }
        
        imports
    }
    
    /// Build AST from analysis results (following C++ adapter pattern)
    fn build_ast_from_analysis(&self, functions: &[FunctionInfo], classes: &[ClassInfo], content: &str) -> crate::core::ast::ASTNode {
        use crate::core::ast::{ASTBuilder, ASTNodeType};
        
        let mut builder = ASTBuilder::new();
        let lines: Vec<&str> = content.lines().collect();
        
        // Classes and their methods (following C++ pattern)
        for class in classes {
            let start_line = self.find_line_number(&lines, &class.name, "class").unwrap_or(class.start_line);
            builder.enter_scope(ASTNodeType::Class, class.name.clone(), start_line);
            
            // Methods within the class body
            for method in &class.methods {
                let method_start = method.start_line.max(start_line);
                builder.add_node(ASTNodeType::Method, method.name.clone(), method_start);
            }
            
            let class_end = class.end_line.max(start_line + 1);
            builder.exit_scope(class_end);
        }
        
        // Top-level functions (exclude class methods)
        for func in functions {
            if matches!(func.metadata.get("is_class_method"), Some(v) if v == "true") {
                continue; // Skip class methods, they're already added above
            }
            
            let func_start = self.find_line_number(&lines, &func.name, "def")
                .unwrap_or(func.start_line.max(1));
            builder.add_node(ASTNodeType::Function, func.name.clone(), func_start);
        }
        
        builder.build()
    }
}

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}