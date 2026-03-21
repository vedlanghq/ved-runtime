pub mod lexer;
pub mod parser;
pub mod ast;
pub mod semantic;
pub mod codegen;

pub fn compile(source_path: &str) {
    println!("Mock compiling: {}", source_path);
    // Placeholder pipeline
    // let tokens = lexer::lex(source);
    // let ast = parser::parse(tokens);
    // semantic::check(&ast);
    // let bytecode = codegen::generate(ast);
    println!("Compilation successful (mock).");
}
