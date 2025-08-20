use tree_sitter::{Parser, Query, QueryCursor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language())?;
    
    let code = r#"
pub fn public_function() {}
pub struct PublicStruct {}
"#;
    
    let tree = parser.parse(code, None).unwrap();
    let root_node = tree.root_node();
    
    // 構造を表示
    print_node(root_node, 0);
    
    // 関数のクエリをテスト
    let test_queries = [
        r#"(function_item visibility: (visibility_modifier)? @vis name: (identifier) @name)"#,
        r#"(function_item vis: (visibility_modifier)? @vis name: (identifier) @name)"#, 
        r#"(function_item visibility_modifier: (visibility_modifier)? @vis name: (identifier) @name)"#,
    ];
    
    for (i, query_str) in test_queries.iter().enumerate() {
        println!("\n=== Testing query {} ===", i + 1);
        println!("Query: {}", query_str);
        
        match Query::new(tree_sitter_rust::language(), query_str) {
            Ok(_) => println!("✅ Query compiled successfully!"),
            Err(e) => println!("❌ Query failed: {:?}", e),
        }
    }
    
    Ok(())
}

fn print_node(node: tree_sitter::Node, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}{}[{}]", indent, node.kind(), node.field_name().unwrap_or(""));
    
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            print_node(child, depth + 1);
        }
    }
}
