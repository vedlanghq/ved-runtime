use crate::lexer::Token;
use crate::ast::*;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) -> &Token {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        self.tokens.get(self.pos - 1).unwrap_or(&Token::EOF)
    }

    fn check(&self, expected: &Token) -> bool {
        self.peek() == expected
    }

    fn consume(&mut self, expected: Token) -> Result<&Token, String> {
        if self.check(&expected) {
            Ok(self.advance())
        } else {
            Err(format!("Syntax Error: Expected {}, but found {}", expected, self.peek()))
        }
    }

    pub fn parse(&mut self) -> Result<Ast, String> {
        let mut statements = Vec::new();

        while !self.check(&Token::EOF) {
            match self.peek() {
                Token::Domain => {
                    statements.push(Statement::DomainDecl(self.parse_domain()?));
                }
                Token::System => {
                    statements.push(Statement::SystemDecl(self.parse_system()?));
                }
                _ => return Err(format!("Unexpected token at top level: {}", self.peek())),
            }
        }

        Ok(Ast { statements })
    }

    fn parse_domain(&mut self) -> Result<DomainDecl, String> {
        self.consume(Token::Domain)?;
        let name = match self.advance() {
            Token::Identifier(id) => id.clone(),
            other => return Err(format!("Expected identifier after 'domain', found {}", other)),
        };

        self.consume(Token::LBrace)?;

        let mut state = Vec::new();
        let mut goals = Vec::new();
        let mut transitions = Vec::new();

        while !self.check(&Token::RBrace) && !self.check(&Token::EOF) {
            match self.peek() {
                Token::State => {
                    state = self.parse_state_block()?;
                }
                Token::Goal => {
                    goals.push(self.parse_goal()?);
                }
                Token::Transition => {
                    transitions.push(self.parse_transition()?);
                }
                other => return Err(format!("Unexpected token in domain body: {}", other)),
            }
        }

        self.consume(Token::RBrace)?;

        Ok(DomainDecl {
            name,
            state,
            goals,
            transitions,
        })
    }

    fn parse_state_block(&mut self) -> Result<Vec<StateField>, String> {
        self.consume(Token::State)?;
        self.consume(Token::LBrace)?;
        let mut fields = Vec::new();
        
        while !self.check(&Token::RBrace) && !self.check(&Token::EOF) {
            let name = match self.advance() {
                Token::Identifier(id) => id.clone(),
                other => return Err(format!("Expected state field name, found {}", other)),
            };
            self.consume(Token::Colon)?;
            let typ = match self.advance() {
                Token::Identifier(id) => id.clone(),
                other => return Err(format!("Expected type for field {}, found {}", name, other)),
            };
            fields.push(StateField { name, typ });
        }
        self.consume(Token::RBrace)?;
        
        Ok(fields)
    }

    fn parse_goal(&mut self) -> Result<GoalDecl, String> {
        self.consume(Token::Goal)?;
        let name = match self.advance() {
            Token::Identifier(id) => id.clone(),
            other => return Err(format!("Expected goal name, found {}", other)),
        };

        self.consume(Token::LBrace)?;
        
        // Support both "target" (original spec) and "predicate" (example usage)
        if self.check(&Token::Target) {
            self.consume(Token::Target)?;
        } else if let Token::Identifier(ref id) = self.peek() {
            if id == "predicate" {
                self.advance();
            } else {
                return Err(format!("Expected 'target' or 'predicate' for goal, found {}", self.peek()));
            }
        } else {
            return Err(format!("Expected 'target' or 'predicate' for goal, found {}", self.peek()));
        }

        let target = self.parse_expression()?;

        // Optional Strategy Block...
        if self.check(&Token::Strategy) {
            self.consume(Token::Strategy)?;
            self.consume(Token::LBrace)?;
            while !self.check(&Token::RBrace) && !self.check(&Token::EOF) {
                self.advance(); // consume strategy config blindly for now
            }
            self.consume(Token::RBrace)?;
        }

        self.consume(Token::RBrace)?;

        Ok(GoalDecl { name, target })
    }

    fn parse_transition(&mut self) -> Result<TransitionDecl, String> {
        self.consume(Token::Transition)?;
        let name = match self.advance() {
            Token::Identifier(id) => id.clone(),
            other => return Err(format!("Expected transition name, found {}", other)),
        };

        self.consume(Token::LBrace)?;
        if self.check(&Token::Slice) {
            self.consume(Token::Slice)?;
        }
        self.consume(Token::Step)?;
        self.consume(Token::LBrace)?;
        
        let mut slice_step = Vec::new();
        while !self.check(&Token::RBrace) && !self.check(&Token::EOF) {
            slice_step.push(self.parse_statement_or_expr()?);
        }
        self.consume(Token::RBrace)?;
        self.consume(Token::RBrace)?;

        Ok(TransitionDecl { name, slice_step })
    }

    fn parse_system(&mut self) -> Result<SystemDecl, String> {
        self.consume(Token::System)?;
        let name = match self.advance() {
            Token::Identifier(id) => id.clone(),
            _ => return Err("Expected system name".to_string()),
        };
        self.consume(Token::LBrace)?;
        
        let mut start_domains = Vec::new();
        while self.check(&Token::Start) {
            self.consume(Token::Start)?;
            self.consume(Token::Domain)?;
            let d_name = match self.advance() {
                Token::Identifier(id) => id.clone(),
                _ => return Err("Expected domain name".to_string()),
            };
            self.consume(Token::LBrace)?;
            let mut init_state = Vec::new();
            while !self.check(&Token::RBrace) {
                init_state.push(self.parse_statement_or_expr()?);
            }
            self.consume(Token::RBrace)?;
            start_domains.push(StartDomain { name: d_name, init_state });
        }
        self.consume(Token::RBrace)?;
        Ok(SystemDecl { name, start_domains })
    }

    fn parse_statement_or_expr(&mut self) -> Result<Expr, String> {
        // A very barebones expression/statement parser
        match self.peek() {
            Token::Send => {
                self.consume(Token::Send)?;
                self.consume(Token::LParen)?;
                let target = match self.advance() {
                    Token::Identifier(id) => id.clone(),
                    Token::StringLiteral(s) => s.clone(),
                    other => return Err(format!("Expected target string or identifier for send, got {}", other)),
                };
                self.consume(Token::Comma)?;
                let message = match self.advance() {
                    Token::Identifier(id) => id.clone(),
                    Token::StringLiteral(s) => s.clone(),
                    other => return Err(format!("Expected message string or identifier for send, got {}", other)),
                };
                self.consume(Token::RParen)?;
                Ok(Expr::Send { target, message })
            }
            Token::SendHigh => {
                self.consume(Token::SendHigh)?;
                self.consume(Token::LParen)?;
                let target = match self.advance() {
                    Token::Identifier(id) => id.clone(),
                    Token::StringLiteral(s) => s.clone(),
                    other => return Err(format!("Expected target string or identifier for send_high, got {}", other)),
                };
                self.consume(Token::Comma)?;
                let message = match self.advance() {
                    Token::Identifier(id) => id.clone(),
                    Token::StringLiteral(s) => s.clone(),
                    other => return Err(format!("Expected message string or identifier for send_high, got {}", other)),
                };
                self.consume(Token::RParen)?;
                Ok(Expr::SendHigh { target, message })
            }
            Token::If => {
                self.consume(Token::If)?;
                let condition = Box::new(self.parse_expression()?);
                self.consume(Token::LBrace)?;
                let mut consequence = Vec::new();
                while !self.check(&Token::RBrace) {
                    consequence.push(self.parse_statement_or_expr()?);
                }
                self.consume(Token::RBrace)?;
                Ok(Expr::If { condition, consequence })
            }
            Token::Identifier(_) => {
                let id = match self.advance() {
                    Token::Identifier(name) => name.clone(),
                    _ => unreachable!(),
                };
                
                if self.check(&Token::Equal) {
                    self.consume(Token::Equal)?;
                    let value = Box::new(self.parse_expression()?);
                    Ok(Expr::Assignment { target: id, value })
                } else {
                    // Let's assume it's part of a binary op or just an indent
                    // Since it's a naive implementation, skip proper precedence parsing for now 
                    if [Token::Plus, Token::Minus, Token::Asterisk, Token::Slash, Token::EqualEqual, Token::LessThan, Token::GreaterThan, Token::GTEqual, Token::LTEqual].contains(self.peek()) {
                        let op = match self.advance() {
                            Token::Plus => "+".to_string(),
                            Token::Minus => "-".to_string(),
                            Token::EqualEqual => "==".to_string(),
                            Token::LessThan => "<".to_string(),
                            Token::GreaterThan => ">".to_string(),
                            Token::GTEqual => ">=".to_string(),
                            Token::LTEqual => "<=".to_string(),
                            _ => return Err("Unsupported op".to_string()),
                        };
                        let right = Box::new(self.parse_expression()?);
                        Ok(Expr::BinaryOp { left: Box::new(Expr::Ident(id)), op, right })
                    } else {
                        Ok(Expr::Ident(id))
                    }
                }
            }
            _ => {
                self.parse_expression()
            }
        }
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        let left = match self.advance() {
            Token::IntLiteral(v) => Expr::IntLiteral(*v),
            Token::StringLiteral(s) => Expr::StringLiteral(s.clone()),
            Token::Identifier(id) => Expr::Ident(id.clone()),
            other => return Err(format!("Unexpected token in expression: {}", other)),
        };

        if [Token::Plus, Token::Minus, Token::Asterisk, Token::Slash, Token::EqualEqual, Token::LessThan, Token::GreaterThan, Token::GTEqual, Token::LTEqual].contains(self.peek()) {
            let op = match self.advance() {
                Token::Plus => "+".to_string(),
                Token::Minus => "-".to_string(),
                Token::EqualEqual => "==".to_string(),
                Token::LessThan => "<".to_string(),
                Token::GreaterThan => ">".to_string(),
                Token::GTEqual => ">=".to_string(),
                Token::LTEqual => "<=".to_string(),
                _ => unreachable!(),
            };
            let right = Box::new(self.parse_expression()?);
            return Ok(Expr::BinaryOp { left: Box::new(left), op, right });
        }

        Ok(left)
    }
}

pub fn parse(input: Vec<Token>) -> Result<Ast, String> {
    let mut parser = Parser::new(input);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    #[test]
    fn test_parse_domain_pseudocode() {
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
        
        let tokens = lex(input);
        let result = parse(tokens);
        
        assert!(result.is_ok(), "Failed to parse AST: {}", result.err().unwrap_or_default());
        
        let ast = result.unwrap();
        assert_eq!(ast.statements.len(), 1);
        
        if let Statement::DomainDecl(domain) = &ast.statements[0] {
            assert_eq!(domain.name, "WebServer");
            assert_eq!(domain.state.len(), 2);
            assert_eq!(domain.state[0].name, "status");
            assert_eq!(domain.state[1].typ, "int");
            
            assert_eq!(domain.goals.len(), 1);
            assert_eq!(domain.goals[0].name, "is_running");
            
            assert_eq!(domain.transitions.len(), 1);
            assert_eq!(domain.transitions[0].name, "start_server");
        } else {
            panic!("Expected DomainDecl statement");
        }
    }
}

