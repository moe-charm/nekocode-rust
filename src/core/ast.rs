//! AST (Abstract Syntax Tree) implementation for NekoCode Rust
//!
//! This module contains the core AST types and functionality, ported from the C++ implementation
//! to provide language-agnostic AST building and manipulation capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AST node types corresponding to C++ ASTNodeType enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ASTNodeType {
    // Basic structure
    #[serde(rename = "file_root")]
    FileRoot,
    #[serde(rename = "namespace")]
    Namespace,
    
    // Classes and structures  
    #[serde(rename = "class")]
    Class,
    #[serde(rename = "struct")]
    Struct,
    #[serde(rename = "interface")]
    Interface,
    #[serde(rename = "enum")]
    Enum,
    
    // Functions and methods
    #[serde(rename = "function")]
    Function,
    #[serde(rename = "method")]
    Method,
    #[serde(rename = "constructor")]
    Constructor,
    #[serde(rename = "destructor")]
    Destructor,
    #[serde(rename = "getter")]
    Getter,
    #[serde(rename = "setter")]
    Setter,
    
    // Variables and properties
    #[serde(rename = "variable")]
    Variable,
    #[serde(rename = "parameter")]
    Parameter,
    #[serde(rename = "property")]
    Property,
    #[serde(rename = "field")]
    Field,
    
    // Control structures
    #[serde(rename = "if_statement")]
    IfStatement,
    #[serde(rename = "else_statement")]
    ElseStatement,
    #[serde(rename = "for_loop")]
    ForLoop,
    #[serde(rename = "while_loop")]
    WhileLoop,
    #[serde(rename = "do_while_loop")]
    DoWhileLoop,
    #[serde(rename = "switch_statement")]
    SwitchStatement,
    #[serde(rename = "case_statement")]
    CaseStatement,
    #[serde(rename = "try_block")]
    TryBlock,
    #[serde(rename = "catch_block")]
    CatchBlock,
    #[serde(rename = "finally_block")]
    FinallyBlock,
    
    // Expressions and calls
    #[serde(rename = "function_call")]
    FunctionCall,
    #[serde(rename = "expression")]
    Expression,
    #[serde(rename = "assignment")]
    Assignment,
    #[serde(rename = "binary_expression")]
    BinaryExpression,
    #[serde(rename = "unary_expression")]
    UnaryExpression,
    
    // Other constructs
    #[serde(rename = "comment")]
    Comment,
    #[serde(rename = "import")]
    Import,
    #[serde(rename = "export")]
    Export,
    #[serde(rename = "block")]
    Block,
    #[serde(rename = "return_statement")]
    ReturnStatement,
    #[serde(rename = "break_statement")]
    BreakStatement,
    #[serde(rename = "continue_statement")]
    ContinueStatement,
    
    #[serde(rename = "unknown")]
    Unknown,
}

impl ASTNodeType {
    /// Convert node type to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ASTNodeType::FileRoot => "file_root",
            ASTNodeType::Namespace => "namespace",
            ASTNodeType::Class => "class",
            ASTNodeType::Struct => "struct",
            ASTNodeType::Interface => "interface",
            ASTNodeType::Enum => "enum",
            ASTNodeType::Function => "function",
            ASTNodeType::Method => "method",
            ASTNodeType::Constructor => "constructor",
            ASTNodeType::Destructor => "destructor",
            ASTNodeType::Getter => "getter",
            ASTNodeType::Setter => "setter",
            ASTNodeType::Variable => "variable",
            ASTNodeType::Parameter => "parameter",
            ASTNodeType::Property => "property",
            ASTNodeType::Field => "field",
            ASTNodeType::IfStatement => "if_statement",
            ASTNodeType::ElseStatement => "else_statement",
            ASTNodeType::ForLoop => "for_loop",
            ASTNodeType::WhileLoop => "while_loop",
            ASTNodeType::DoWhileLoop => "do_while_loop",
            ASTNodeType::SwitchStatement => "switch_statement",
            ASTNodeType::CaseStatement => "case_statement",
            ASTNodeType::TryBlock => "try_block",
            ASTNodeType::CatchBlock => "catch_block",
            ASTNodeType::FinallyBlock => "finally_block",
            ASTNodeType::FunctionCall => "function_call",
            ASTNodeType::Expression => "expression",
            ASTNodeType::Assignment => "assignment",
            ASTNodeType::BinaryExpression => "binary_expression",
            ASTNodeType::UnaryExpression => "unary_expression",
            ASTNodeType::Comment => "comment",
            ASTNodeType::Import => "import",
            ASTNodeType::Export => "export",
            ASTNodeType::Block => "block",
            ASTNodeType::ReturnStatement => "return_statement",
            ASTNodeType::BreakStatement => "break_statement",
            ASTNodeType::ContinueStatement => "continue_statement",
            ASTNodeType::Unknown => "unknown",
        }
    }
}

/// AST Node representing a single element in the syntax tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASTNode {
    // Basic node information
    #[serde(rename = "type")]
    pub node_type: ASTNodeType,
    pub name: String,
    pub full_name: String,
    
    // Position information
    pub start_line: u32,
    pub end_line: u32,
    pub start_column: u32,
    pub end_column: u32,
    
    // Hierarchy information
    pub depth: u32,
    pub scope_path: String,
    
    // Tree structure (note: we use indices instead of references for serialization)
    pub children: Vec<ASTNode>,
    
    // Additional metadata
    pub attributes: HashMap<String, String>,
    pub source_text: Option<String>,
}

impl ASTNode {
    /// Create a new AST node
    pub fn new(node_type: ASTNodeType, name: String) -> Self {
        Self {
            node_type,
            name: name.clone(),
            full_name: name,
            start_line: 0,
            end_line: 0,
            start_column: 0,
            end_column: 0,
            depth: 0,
            scope_path: String::new(),
            children: Vec::new(),
            attributes: HashMap::new(),
            source_text: None,
        }
    }
    
    /// Add a child node
    pub fn add_child(&mut self, mut child: ASTNode) {
        child.depth = self.depth + 1;
        child.scope_path = self.build_scope_path(&child.name);
        self.children.push(child);
    }
    
    /// Build scope path for a child
    fn build_scope_path(&self, child_name: &str) -> String {
        if self.scope_path.is_empty() {
            child_name.to_string()
        } else {
            format!("{}::{}", self.scope_path, child_name)
        }
    }
    
    /// Find children by type
    pub fn find_children_by_type(&self, target_type: ASTNodeType) -> Vec<&ASTNode> {
        self.children
            .iter()
            .filter(|child| child.node_type == target_type)
            .collect()
    }
    
    /// Find all descendants by type (recursive)
    pub fn find_descendants_by_type(&self, target_type: ASTNodeType) -> Vec<&ASTNode> {
        let mut result = Vec::new();
        self.find_descendants_recursive(target_type, &mut result);
        result
    }
    
    /// Recursive helper for finding descendants
    fn find_descendants_recursive<'a>(&'a self, target_type: ASTNodeType, result: &mut Vec<&'a ASTNode>) {
        for child in &self.children {
            if child.node_type == target_type {
                result.push(child);
            }
            child.find_descendants_recursive(target_type, result);
        }
    }
    
    /// Query nodes by path (e.g., "MyClass::myMethod")
    pub fn query_by_path(&self, path: &str) -> Vec<&ASTNode> {
        let mut result = Vec::new();
        self.query_by_path_recursive(path, &mut result);
        result
    }
    
    /// Recursive helper for path queries
    fn query_by_path_recursive<'a>(&'a self, path: &str, result: &mut Vec<&'a ASTNode>) {
        if self.scope_path == path || self.full_name == path {
            result.push(self);
        }
        
        for child in &self.children {
            child.query_by_path_recursive(path, result);
        }
    }
    
    /// Find the deepest node at a specific line
    pub fn find_node_at_line(&self, line: u32) -> Option<&ASTNode> {
        // Check if this node contains the line
        if line >= self.start_line && line <= self.end_line {
            // Check children first (deepest wins)
            for child in &self.children {
                if let Some(deeper) = child.find_node_at_line(line) {
                    return Some(deeper);
                }
            }
            // If no child contains the line, this node is the deepest
            Some(self)
        } else {
            None
        }
    }
    
    /// Get node type as string
    pub fn type_string(&self) -> &'static str {
        self.node_type.as_str()
    }
    
    /// Dump AST as tree format
    pub fn dump_as_tree(&self, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        let mut result = format!(
            "{}{} '{}' ({}:{}-{}:{})",
            indent_str,
            self.type_string(),
            self.name,
            self.start_line,
            self.start_column,
            self.end_line,
            self.end_column
        );
        
        if !self.attributes.is_empty() {
            result.push_str(&format!(" {:?}", self.attributes));
        }
        
        result.push('\n');
        
        for child in &self.children {
            result.push_str(&child.dump_as_tree(indent + 1));
        }
        
        result
    }
    
    /// Dump AST as flat list
    pub fn dump_as_flat(&self) -> String {
        let mut result = Vec::new();
        self.collect_flat_recursive(&mut result);
        result.join("\n")
    }
    
    /// Recursive helper for flat dump
    fn collect_flat_recursive(&self, result: &mut Vec<String>) {
        result.push(format!(
            "{} '{}' at {}:{} scope='{}'",
            self.type_string(),
            self.name,
            self.start_line,
            self.start_column,
            self.scope_path
        ));
        
        for child in &self.children {
            child.collect_flat_recursive(result);
        }
    }
}

/// AST Statistics structure matching C++ ASTStatistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASTStatistics {
    pub total_nodes: u32,
    pub max_depth: u32,
    pub node_type_counts: HashMap<String, u32>,
    pub classes: u32,
    pub functions: u32,
    pub methods: u32,
    pub variables: u32,
    pub control_structures: u32,
}

impl Default for ASTStatistics {
    fn default() -> Self {
        Self {
            total_nodes: 0,
            max_depth: 0,
            node_type_counts: HashMap::new(),
            classes: 0,
            functions: 0,
            methods: 0,
            variables: 0,
            control_structures: 0,
        }
    }
}

impl ASTStatistics {
    /// Update statistics from AST root
    pub fn update_from_root(&mut self, root: &ASTNode) {
        *self = Self::default();
        let new_stats = self.collect_statistics_recursive(root);
        *self = new_stats;
    }
    
    /// Recursive statistics collection
    fn collect_statistics_recursive(&self, node: &ASTNode) -> Self {
        let mut stats = Self::default();
        
        stats.total_nodes = 1;
        stats.max_depth = node.depth;
        
        // Count by type
        let type_key = node.type_string().to_string();
        stats.node_type_counts.insert(type_key, 1);
        
        // Category counts
        match node.node_type {
            ASTNodeType::Class | ASTNodeType::Struct | ASTNodeType::Interface => {
                stats.classes = 1;
            }
            ASTNodeType::Function => {
                stats.functions = 1;
            }
            ASTNodeType::Method | ASTNodeType::Constructor | ASTNodeType::Destructor => {
                stats.methods = 1;
            }
            ASTNodeType::Variable | ASTNodeType::Parameter | ASTNodeType::Property | ASTNodeType::Field => {
                stats.variables = 1;
            }
            ASTNodeType::IfStatement | ASTNodeType::ForLoop | ASTNodeType::WhileLoop | 
            ASTNodeType::SwitchStatement | ASTNodeType::TryBlock => {
                stats.control_structures = 1;
            }
            _ => {}
        }
        
        // Process children
        for child in &node.children {
            let child_stats = self.collect_statistics_recursive(child);
            stats.merge(child_stats);
        }
        
        stats
    }
    
    /// Merge statistics from another instance
    fn merge(&mut self, other: Self) {
        self.total_nodes += other.total_nodes;
        self.max_depth = self.max_depth.max(other.max_depth);
        self.classes += other.classes;
        self.functions += other.functions;
        self.methods += other.methods;
        self.variables += other.variables;
        self.control_structures += other.control_structures;
        
        for (key, count) in other.node_type_counts {
            *self.node_type_counts.entry(key).or_insert(0) += count;
        }
    }
}

/// AST Builder for constructing syntax trees
pub struct ASTBuilder {
    root: ASTNode,
    current_scope: Vec<usize>, // Path to current scope (indices in tree)
}

impl ASTBuilder {
    /// Create a new AST builder
    pub fn new() -> Self {
        Self {
            root: ASTNode::new(ASTNodeType::FileRoot, "".to_string()),
            current_scope: vec![],
        }
    }
    
    /// Enter a new scope
    pub fn enter_scope(&mut self, node_type: ASTNodeType, name: String, line: u32) {
        let mut node = ASTNode::new(node_type, name);
        node.start_line = line;
        
        // Add to current scope
        self.get_current_scope_mut().add_child(node);
        
        // Update current scope path
        let new_index = self.get_current_scope().children.len() - 1;
        self.current_scope.push(new_index);
    }
    
    /// Exit current scope
    pub fn exit_scope(&mut self, end_line: u32) {
        if !self.current_scope.is_empty() {
            // Set end line for current scope
            self.get_current_scope_mut().end_line = end_line;
            
            // Pop from scope stack
            self.current_scope.pop();
        }
    }
    
    /// Add a node to current scope
    pub fn add_node(&mut self, node_type: ASTNodeType, name: String, line: u32) {
        let mut node = ASTNode::new(node_type, name);
        node.start_line = line;
        node.end_line = line;
        
        self.get_current_scope_mut().add_child(node);
    }
    
    /// Get the current scope node
    fn get_current_scope(&self) -> &ASTNode {
        let mut current = &self.root;
        for &index in &self.current_scope {
            current = &current.children[index];
        }
        current
    }
    
    /// Get the current scope node (mutable)
    fn get_current_scope_mut(&mut self) -> &mut ASTNode {
        let mut current = &mut self.root;
        for &index in &self.current_scope {
            current = &mut current.children[index];
        }
        current
    }
    
    /// Build and return the final AST
    pub fn build(self) -> ASTNode {
        self.root
    }
}

impl Default for ASTBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ast_node_creation() {
        let node = ASTNode::new(ASTNodeType::Class, "TestClass".to_string());
        assert_eq!(node.node_type, ASTNodeType::Class);
        assert_eq!(node.name, "TestClass");
        assert_eq!(node.depth, 0);
    }
    
    #[test]
    fn test_ast_builder() {
        let mut builder = ASTBuilder::new();
        builder.enter_scope(ASTNodeType::Class, "MyClass".to_string(), 1);
        builder.add_node(ASTNodeType::Method, "myMethod".to_string(), 2);
        builder.exit_scope(10);
        
        let ast = builder.build();
        assert_eq!(ast.children.len(), 1);
        assert_eq!(ast.children[0].name, "MyClass");
        assert_eq!(ast.children[0].children.len(), 1);
        assert_eq!(ast.children[0].children[0].name, "myMethod");
    }
    
    #[test]
    fn test_ast_query() {
        let mut root = ASTNode::new(ASTNodeType::FileRoot, "".to_string());
        let mut class_node = ASTNode::new(ASTNodeType::Class, "MyClass".to_string());
        class_node.scope_path = "MyClass".to_string();
        
        let mut method_node = ASTNode::new(ASTNodeType::Method, "myMethod".to_string());
        method_node.scope_path = "MyClass::myMethod".to_string();
        
        class_node.add_child(method_node);
        root.add_child(class_node);
        
        let results = root.query_by_path("MyClass::myMethod");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "myMethod");
    }
}