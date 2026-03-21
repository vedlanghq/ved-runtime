#[derive(Debug, Clone)]
pub enum Instruction {
    LoadConst(i64),
    LoadState(String),
    StoreState(String),
    Add,
    Sub,
    CmpEq,
    JumpIf(usize),
    Jump(usize),
    SendMsg(String),
    HaltSlice,
}
