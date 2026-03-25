#[derive(Debug, Clone)]
pub enum Statement {
    DomainDecl(DomainDecl),
    SystemDecl(SystemDecl),
}

#[derive(Debug, Clone)]
pub struct DomainDecl {
    pub name: String,
    pub state: Vec<StateField>,
    pub goals: Vec<GoalDecl>,
    pub transitions: Vec<TransitionDecl>,
}

#[derive(Debug, Clone)]
pub struct StateField {
    pub name: String,
    pub typ: String, // e.g. "int"
}

#[derive(Debug, Clone)]
pub struct GoalDecl {
    pub name: String,
    pub target: Expr,
}

#[derive(Debug, Clone)]
pub struct TransitionDecl {
    pub name: String,
    pub slice_step: Vec<Expr>, // Using Expr for block statements for now
}

#[derive(Debug, Clone)]
pub struct SystemDecl {
    pub name: String,
    pub start_domains: Vec<StartDomain>,
}

#[derive(Debug, Clone)]
pub struct StartDomain {
    pub name: String,
    pub init_state: Vec<Expr>, // Initialization assignments
}

#[derive(Debug, Clone)]
pub enum Expr {
    BinaryOp {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Assignment {
        target: String,
        value: Box<Expr>,
    },
    Ident(String),
    IntLiteral(i64),
    StringLiteral(String),
    Send {
        target: String,
        message: String,
    },
    SendHigh {
        target: String,
        message: String,
    },
    If {
        condition: Box<Expr>,
        consequence: Vec<Expr>,
    },
}

#[derive(Debug, Clone)]
pub struct Ast {
    pub statements: Vec<Statement>,
}
