pub mod lexer;
pub mod parser;
pub mod ast;
pub mod semantic;
pub mod codegen;

pub fn compile_source(source: &str) -> Result<codegen::BytecodeProgram, String> {
    let tokens = lexer::lex(source);
    // Simple basic lexing check
    for t in &tokens {
        if let lexer::Token::Unknown(c) = t {
            return Err(format!("Unknown character: {}", c));
        }
    }

    let ast = parser::parse(tokens)?;

    let mut validator = semantic::SemanticValidator::new();
    if let Err(errors) = validator.validate(&ast) {
        let err_msgs: Vec<String> = errors.into_iter().map(|e| e.message).collect();
        return Err(format!("Semantic Errors:\n{}", err_msgs.join("\n")));
    }

    let generator = codegen::CodeGenerator::new();
    let program = generator.generate(&ast);

    Ok(program)
}
