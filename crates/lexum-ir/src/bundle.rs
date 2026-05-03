use crate::instruction::Instruction;

#[derive(Debug, Clone)]
pub struct Transition {
    pub name: String,
    pub code: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct VedBundle {
    pub transitions: Vec<Transition>,
}
