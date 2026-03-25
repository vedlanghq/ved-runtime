use crate::ast::{Ast, Expr, Statement, DomainDecl};
use std::collections::HashMap;
pub use ved_ir::bytecode::*;

pub struct CodeGenerator {
    program: BytecodeProgram,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            program: BytecodeProgram { domains: Vec::new() },
        }
    }

    pub fn generate(mut self, ast: &Ast) -> BytecodeProgram {
        for stmt in &ast.statements {
            if let Statement::DomainDecl(domain) = stmt {
                let domain_code = self.generate_domain(domain);
                self.program.domains.push(domain_code);
            }
        }
        self.program
    }

    fn generate_domain(&self, domain: &DomainDecl) -> DomainBytecode {
        let mut state_schema = Vec::new();
        let mut field_map = HashMap::new();

        for (idx, field) in domain.state.iter().enumerate() {
            state_schema.push(field.name.clone());
            field_map.insert(field.name.clone(), idx);
        }

        let mut transitions = Vec::new();
        for trans in &domain.transitions {
            let mut cg = FuncGen::new(&field_map);
            for step in &trans.slice_step {
                cg.compile_expr(step);
            }
            cg.emit(OpCode::HaltSlice);
            transitions.push(TransitionBytecode {
                name: trans.name.clone(),
                constants: cg.constants,
                instructions: cg.instructions,
            });
        }

        let mut goals = Vec::new();
        for goal in &domain.goals {
            let mut cg = FuncGen::new(&field_map);
            let _res_reg = cg.compile_expr(&goal.target);
            // goals typically return a boolean, so the result is left in `res_reg`
            cg.emit(OpCode::HaltSlice);
            goals.push(GoalBytecode {
                name: goal.name.clone(),
                constants: cg.constants,
                instructions: cg.instructions,
            });
        }

        DomainBytecode {
            name: domain.name.clone(),
            state_schema,
            transitions,
            goals,
        }
    }
}

/// Helper context for generating a single sequence of instructions
struct FuncGen<'a> {
    field_map: &'a HashMap<String, usize>,
    instructions: Vec<OpCode>,
    constants: Vec<Constant>,
    next_reg: u8,
}

impl<'a> FuncGen<'a> {
    fn new(field_map: &'a HashMap<String, usize>) -> Self {
        FuncGen {
            field_map,
            instructions: Vec::new(),
            constants: Vec::new(),
            next_reg: 0,
        }
    }

    fn emit(&mut self, op: OpCode) {
        self.instructions.push(op);
    }

    fn alloc_reg(&mut self) -> u8 {
        let r = self.next_reg;
        self.next_reg += 1;
        r
    }

    fn add_constant(&mut self, val: Constant) -> usize {
        if let Some(pos) = self.constants.iter().position(|c| c == &val) {
            return pos;
        }
        let pos = self.constants.len();
        self.constants.push(val);
        pos
    }

    /// Compiles an expression and returns the register containing its result
    fn compile_expr(&mut self, expr: &Expr) -> u8 {
        match expr {
            Expr::IntLiteral(v) => {
                let const_idx = self.add_constant(Constant::Int(*v));
                let dest_reg = self.alloc_reg();
                self.emit(OpCode::LoadConst { const_idx, dest_reg });
                dest_reg
            }
            Expr::StringLiteral(v) => {
                let const_idx = self.add_constant(Constant::String(v.clone()));
                let dest_reg = self.alloc_reg();
                self.emit(OpCode::LoadConst { const_idx, dest_reg });
                dest_reg
            }
            Expr::Ident(name) => {
                let dest_reg = self.alloc_reg();
                if let Some(&field_idx) = self.field_map.get(name) {
                    self.emit(OpCode::LoadState { field_idx, dest_reg });
                } else {
                    // Locals not handled in v0.1 pseudo-ISA perfectly, fallback 
                    // Let semantic check catch real unregistered vars
                }
                dest_reg
            }
            Expr::Assignment { target, value } => {
                let src_reg = self.compile_expr(value);
                if let Some(&field_idx) = self.field_map.get(target) {
                    self.emit(OpCode::StoreState { src_reg, field_idx });
                }
                src_reg
            }
            Expr::BinaryOp { left, op, right } => {
                let r1 = self.compile_expr(left);
                let r2 = self.compile_expr(right);
                let dest = self.alloc_reg();

                let opcode = match op.as_str() {
                    "+" => OpCode::AddInt { r1, r2, dest },
                    "-" => OpCode::SubInt { r1, r2, dest },
                    "==" => OpCode::CmpEq { r1, r2, dest },
                    "<" => OpCode::CmpLt { r1, r2, dest },
                    ">" => OpCode::CmpGt { r1, r2, dest },
                    ">=" => OpCode::CmpGte { r1, r2, dest },
                    "<=" => OpCode::CmpLte { r1, r2, dest },
                    _ => unimplemented!("unsupported binary op: {}", op),
                };
                self.emit(opcode);
                dest
            }
            Expr::If { condition, consequence } => {
                let cond_reg = self.compile_expr(condition);
                
                // Placeholder for jump - we emit a jumpiffalse, then patch its offset later
                let jmp_idx = self.instructions.len();
                self.emit(OpCode::JumpIfFalse { test_reg: cond_reg, target_offset: 0 }); // offset 0 for now
                
                for step in consequence {
                    self.compile_expr(step);
                }
                
                let target = self.instructions.len();
                if let OpCode::JumpIfFalse { target_offset, .. } = &mut self.instructions[jmp_idx] {
                    *target_offset = target;
                }
                
                cond_reg // returns condition bool register for now...
            }
            Expr::Send { target, message } => {
                let target_const_idx = self.add_constant(Constant::String(target.clone()));
                let msg_const_idx = self.add_constant(Constant::String(message.clone()));
                self.emit(OpCode::SendMsg { target_const_idx, msg_const_idx });
                self.alloc_reg() // Return dummy register
            }
            Expr::SendHigh { target, message } => {
                let target_const_idx = self.add_constant(Constant::String(target.clone()));
                let msg_const_idx = self.add_constant(Constant::String(message.clone()));
                self.emit(OpCode::SendHighMsg { target_const_idx, msg_const_idx });
                self.alloc_reg() // Return dummy register
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;
    use crate::semantic::SemanticValidator;

    #[test]
    fn test_codegen_basic() {
        let input = r#"
        domain WebServer {
            state {
                status: string
                port: int
            }
            goal is_running {
                target status == "online"
            }
            transition start_server {
                slice step {
                    status = "online"
                    port = port + 1
                }
            }
        }
        "#;
        let ast = parse(lex(input)).unwrap();
        let mut validator = SemanticValidator::new();
        validator.validate(&ast).unwrap();

        let codegen = CodeGenerator::new();
        let program = codegen.generate(&ast);
        
        assert_eq!(program.domains.len(), 1);
        let domain = &program.domains[0];
        
        assert_eq!(domain.name, "WebServer");
        assert_eq!(domain.state_schema, vec!["status".to_string(), "port".to_string()]);
        
        // Check goals bytecode
        let goal = &domain.goals[0];
        assert_eq!(goal.name, "is_running");
        assert!(goal.instructions.len() > 0);
        
        // Check transition bytecode
        let trans = &domain.transitions[0];
        assert_eq!(trans.name, "start_server");
        
        // Contains loading constants, state reads/writes, math, halt
        assert!(matches!(trans.instructions.last().unwrap(), OpCode::HaltSlice));
    }
}

