#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Int(i64),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    LoadConst { const_idx: usize, dest_reg: u8 },
    LoadState { field_idx: usize, dest_reg: u8 },
    StoreState { src_reg: u8, field_idx: usize },
    AddInt { r1: u8, r2: u8, dest: u8 },
    SubInt { r1: u8, r2: u8, dest: u8 },
    CmpEq { r1: u8, r2: u8, dest: u8 },
    CmpLt { r1: u8, r2: u8, dest: u8 },
    CmpGt { r1: u8, r2: u8, dest: u8 },
    CmpGte { r1: u8, r2: u8, dest: u8 },
    CmpLte { r1: u8, r2: u8, dest: u8 },
    JumpIfFalse { test_reg: u8, target_offset: usize },
    Jump { target_offset: usize },
    SendMsg { target_const_idx: usize, msg_const_idx: usize },
    HaltSlice,
}

#[derive(Debug, Clone)]
pub struct TransitionBytecode {
    pub name: String,
    pub constants: Vec<Constant>,
    pub instructions: Vec<OpCode>,
}

#[derive(Debug, Clone)]
pub struct GoalBytecode {
    pub name: String,
    pub constants: Vec<Constant>,
    pub instructions: Vec<OpCode>,
}

#[derive(Debug, Clone)]
pub struct DomainBytecode {
    pub name: String,
    pub state_schema: Vec<String>,
    pub transitions: Vec<TransitionBytecode>,
    pub goals: Vec<GoalBytecode>,
}

#[derive(Debug, Clone)]
pub struct BytecodeProgram {
    pub domains: Vec<DomainBytecode>,
}
