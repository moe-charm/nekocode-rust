//! AST (Abstract Syntax Tree) utilities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tree_sitter::{Node, Tree};

use nekocode_core::{Result, NekocodeError};

/// AST node type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ASTNodeType {
    Program,
    Function,
    Class,
    Method,
    Variable,
    Import,
    Export,
    IfStatement,
    ForLoop,
    WhileLoop,
    SwitchStatement,
    TryStatement,
    Block,
    Expression,
    Identifier,
    Literal,
    Comment,
    Other(String),
}

/// AST node representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASTNode {
    pub node_type: ASTNodeType,
    pub text: String,
    pub start_line: u32,
    pub end_line: u32,
    pub start_column: u32,
    pub end_column: u32,
    pub children: Vec<ASTNode>,
    pub metadata: HashMap<String, String>,
}

impl ASTNode {
    /// Create a new AST node
    pub fn new(node_type: ASTNodeType) -> Self {
        Self {
            node_type,
            text: String::new(),
            start_line: 0,
            end_line: 0,
            start_column: 0,
            end_column: 0,
            children: Vec::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// Create from tree-sitter node
    pub fn from_tree_sitter_node(node: Node, source: &str) -> Result<Self> {
        let mut ast_node = Self::new(Self::map_node_kind(node.kind()));
        
        ast_node.start_line = node.start_position().row as u32 + 1;
        ast_node.end_line = node.end_position().row as u32 + 1;
        ast_node.start_column = node.start_position().column as u32;
        ast_node.end_column = node.end_position().column as u32;
        
        // Get text for leaf nodes
        if node.child_count() == 0 {
            ast_node.text = node.utf8_text(source.as_bytes())
                .map_err(|e| NekocodeError::Analysis(format!("UTF8 error: {}", e)))?
                .to_string();
        }
        
        // Recursively process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                ast_node.children.push(Self::from_tree_sitter_node(child, source)?);
            }
        }
        
        Ok(ast_node)
    }
    
    /// Map tree-sitter node kind to AST node type
    fn map_node_kind(kind: &str) -> ASTNodeType {
        match kind {
            "program" | "source_file" | "module" => ASTNodeType::Program,
            "function_declaration" | "function_definition" | "function_item" => ASTNodeType::Function,
            "class_declaration" | "class_definition" => ASTNodeType::Class,
            "method_definition" | "method_declaration" => ASTNodeType::Method,
            "variable_declaration" | "let_declaration" | "const_declaration" => ASTNodeType::Variable,
            "import_statement" | "import_declaration" | "use_declaration" => ASTNodeType::Import,
            "export_statement" | "export_declaration" => ASTNodeType::Export,
            "if_statement" | "if_expression" => ASTNodeType::IfStatement,
            "for_statement" | "for_in_statement" | "for_of_statement" => ASTNodeType::ForLoop,
            "while_statement" | "while_expression" => ASTNodeType::WhileLoop,
            "switch_statement" | "match_expression" => ASTNodeType::SwitchStatement,
            "try_statement" | "try_expression" => ASTNodeType::TryStatement,
            "block" | "block_statement" | "statement_block" => ASTNodeType::Block,
            "expression" | "expression_statement" => ASTNodeType::Expression,
            "identifier" => ASTNodeType::Identifier,
            "string_literal" | "number_literal" | "boolean_literal" | "null_literal" => ASTNodeType::Literal,
            "comment" | "line_comment" | "block_comment" => ASTNodeType::Comment,
            _ => ASTNodeType::Other(kind.to_string()),
        }
    }
    
    /// Count nodes of a specific type
    pub fn count_node_type(&self, node_type: &ASTNodeType) -> usize {
        let mut count = 0;
        if &self.node_type == node_type {
            count += 1;
        }
        for child in &self.children {
            count += child.count_node_type(node_type);
        }
        count
    }
    
    /// Find all nodes of a specific type
    pub fn find_nodes_by_type(&self, node_type: &ASTNodeType) -> Vec<&ASTNode> {
        let mut nodes = Vec::new();
        if &self.node_type == node_type {
            nodes.push(self);
        }
        for child in &self.children {
            nodes.extend(child.find_nodes_by_type(node_type));
        }
        nodes
    }
    
    /// Calculate depth of the AST
    pub fn depth(&self) -> usize {
        if self.children.is_empty() {
            1
        } else {
            1 + self.children.iter().map(|c| c.depth()).max().unwrap_or(0)
        }
    }
}

/// AST statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASTStatistics {
    pub total_nodes: usize,
    pub max_depth: usize,
    pub node_counts: HashMap<String, usize>,
    pub complexity_score: u32,
}

impl ASTStatistics {
    /// Calculate statistics from an AST
    pub fn from_ast(ast: &ASTNode) -> Self {
        let mut stats = Self {
            total_nodes: 0,
            max_depth: ast.depth(),
            node_counts: HashMap::new(),
            complexity_score: 0,
        };
        
        stats.calculate_node_counts(ast);
        stats.calculate_complexity(ast);
        
        stats
    }
    
    fn calculate_node_counts(&mut self, node: &ASTNode) {
        self.total_nodes += 1;
        
        let type_name = match &node.node_type {
            ASTNodeType::Other(s) => s.clone(),
            _ => format!("{:?}", node.node_type),
        };
        
        *self.node_counts.entry(type_name).or_insert(0) += 1;
        
        for child in &node.children {
            self.calculate_node_counts(child);
        }
    }
    
    fn calculate_complexity(&mut self, node: &ASTNode) {
        // Complexity points for control flow structures
        match &node.node_type {
            ASTNodeType::IfStatement => self.complexity_score += 1,
            ASTNodeType::ForLoop | ASTNodeType::WhileLoop => self.complexity_score += 2,
            ASTNodeType::SwitchStatement => self.complexity_score += 2,
            ASTNodeType::TryStatement => self.complexity_score += 1,
            _ => {}
        }
        
        for child in &node.children {
            self.calculate_complexity(child);
        }
    }
}

/// AST builder for constructing AST from Tree-sitter trees
pub struct ASTBuilder;

impl ASTBuilder {
    /// Build AST from Tree-sitter tree
    pub fn build_from_tree(tree: &Tree, source: &str) -> Result<ASTNode> {
        ASTNode::from_tree_sitter_node(tree.root_node(), source)
    }
    
    /// Build and get statistics in one pass
    pub fn build_with_stats(tree: &Tree, source: &str) -> Result<(ASTNode, ASTStatistics)> {
        let ast = Self::build_from_tree(tree, source)?;
        let stats = ASTStatistics::from_ast(&ast);
        Ok((ast, stats))
    }
    
    /// Query AST nodes by path (e.g., "MyClass::myMethod")
    pub fn query_by_path<'a>(ast: &'a ASTNode, path: &str) -> Option<&'a ASTNode> {
        let parts: Vec<&str> = path.split("::").collect();
        Self::find_by_path_parts(ast, &parts)
    }
    
    fn find_by_path_parts<'a>(node: &'a ASTNode, parts: &[&str]) -> Option<&'a ASTNode> {
        if parts.is_empty() {
            return Some(node);
        }
        
        let target = parts[0];
        
        // Check if current node matches
        if node.text == target || 
           (node.metadata.get("name").map(|n| n.as_str()) == Some(target)) {
            if parts.len() == 1 {
                return Some(node);
            } else {
                // Continue searching in children
                for child in &node.children {
                    if let Some(found) = Self::find_by_path_parts(child, &parts[1..]) {
                        return Some(found);
                    }
                }
            }
        }
        
        // Search in all children
        for child in &node.children {
            if let Some(found) = Self::find_by_path_parts(child, parts) {
                return Some(found);
            }
        }
        
        None
    }
}