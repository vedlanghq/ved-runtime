use std::collections::HashMap;
use crate::ast::{Ast, Statement, Expr};

#[derive(Debug, Clone, PartialEq)]
pub enum VedType {
    Int,
    String,
    Bool,
    Unknown(String),
}

#[derive(Debug)]
pub struct SemanticError {
    pub message: String,
}

pub struct SemanticValidator {
    domains: HashMap<String, DomainInfo>,
}

struct DomainInfo {
    state_fields: HashMap<String, VedType>,
}

impl SemanticValidator {
    pub fn new() -> Self {
        SemanticValidator {
            domains: HashMap::new(),
        }
    }

    pub fn validate(&mut self, ast: &Ast) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        // Pass 1: Catalog all domain states
        for stmt in &ast.statements {
            if let Statement::DomainDecl(domain) = stmt {
                let mut state_fields = HashMap::new();
                for field in &domain.state {
                    let v_type = match field.typ.as_str() {
                        "int" => VedType::Int,
                        "string" => VedType::String,
                        "bool" => VedType::Bool,
                        other => VedType::Unknown(other.to_string()),
                    };
                    
                    if let VedType::Unknown(ref t) = v_type {
                        errors.push(SemanticError {
                            message: format!("Domain '{}': Unknown type '{}' for field '{}'", domain.name, t, field.name),
                        });
                    }

                    if state_fields.contains_key(&field.name) {
                        errors.push(SemanticError {
                            message: format!("Domain '{}': Duplicate state field '{}'", domain.name, field.name),
                        });
                    } else {
                        state_fields.insert(field.name.clone(), v_type);
                    }
                }

                self.domains.insert(domain.name.clone(), DomainInfo { state_fields });
            }
        }

        // Pass 2: Validate Goals and Transitions against State
        for stmt in &ast.statements {
            if let Statement::DomainDecl(domain) = stmt {
                let domain_info = self.domains.get(&domain.name).unwrap();

                // Validate Goals
                for goal in &domain.goals {
                    self.validate_expr(&domain.name, &goal.target, domain_info, &mut errors);
                }

                // Validate Transitions
                for transition in &domain.transitions {
                    for expr in &transition.slice_step {
                        self.validate_expr(&domain.name, &expr, domain_info, &mut errors);
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_expr(&self, domain_name: &str, expr: &Expr, domain_info: &DomainInfo, errors: &mut Vec<SemanticError>) {
        match expr {
            Expr::Ident(name) => {
                if !domain_info.state_fields.contains_key(name) {
                    errors.push(SemanticError {
                        message: format!("Domain '{}': Reference to undefined state variable '{}'", domain_name, name),
                    });
                }
            }
            Expr::Assignment { target, value } => {
                if !domain_info.state_fields.contains_key(target) {
                    errors.push(SemanticError {
                        message: format!("Domain '{}': Cannot assign to undefined state variable '{}'", domain_name, target),
                    });
                }
                self.validate_expr(domain_name, value, domain_info, errors);
                // Future consideration: Add type checking here (e.g., target type == value type)
            }
            Expr::BinaryOp { left, right, .. } => {
                self.validate_expr(domain_name, left, domain_info, errors);
                self.validate_expr(domain_name, right, domain_info, errors);
            }
            Expr::If { condition, consequence } => {
                self.validate_expr(domain_name, condition, domain_info, errors);
                for step in consequence {
                    self.validate_expr(domain_name, step, domain_info, errors);
                }
            }
            Expr::Send { target: _, message: _ } => {
                // Ensure target domain exists? Could be checked here eventually.
            }
            Expr::SendHigh { target: _, message: _ } => {
                // High priority send
            }
            Expr::IntLiteral(_) | Expr::StringLiteral(_) => {
                // Literals are inherently valid.
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    #[test]
    fn test_valid_semantic() {
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
                }
            }
        }
        "#;
        let ast = parse(lex(input)).unwrap();
        let mut validator = SemanticValidator::new();
        let result = validator.validate(&ast);
        assert!(result.is_ok(), "Should pass semantic validation");
    }

    #[test]
    fn test_invalid_semantic_variable() {
        let input = r#"
        domain WebServer {
            state {
                port: int
            }
            transition start_server {
                slice step {
                    status = "online"
                }
            }
        }
        "#;
        let ast = parse(lex(input)).unwrap();
        let mut validator = SemanticValidator::new();
        let result = validator.validate(&ast);
        assert!(result.is_err());
        let errors = result.err().unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("undefined state variable 'status'"));
    }
}


