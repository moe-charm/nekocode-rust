use nekocode_rust::core::ast::{ASTBuilder, ASTNodeType};

fn main() {
    let mut builder = ASTBuilder::new();
    
    // Enter class scope
    builder.enter_scope(ASTNodeType::Class, "System".to_string(), 1);
    
    // Enter method scope
    builder.enter_scope(ASTNodeType::Method, "getAccessibleFileSystemEntries".to_string(), 2);
    
    // Exit method scope
    builder.exit_scope(3);
    
    // Exit class scope
    builder.exit_scope(4);
    
    let ast = builder.build();
    
    // Print the structure and check scope paths
    println!("AST structure:");
    print_ast(&ast, 0);
}

fn print_ast(node: &nekocode_rust::core::ast::ASTNode, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}{}('{}') scope_path='{}'", 
             indent, 
             node.type_string(), 
             node.name, 
             node.scope_path);
    
    for child in &node.children {
        print_ast(child, depth + 1);
    }
}