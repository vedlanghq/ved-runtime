use ved_ir::bytecode::{OpCode, Constant, TransitionBytecode};
use crate::messaging::Message;
use crate::state::IsolatedState;

pub struct Interpreter {
    pub state: IsolatedState,
    pub registers: [i64; 256],
}

impl Interpreter {
    pub fn new(schema: &[String]) -> Self {
        Self {
            state: IsolatedState::new(schema),
            registers: [0; 256],
        }
    }

    pub fn with_state(state: IsolatedState) -> Self {
        Self {
            state,
            registers: [0; 256],
        }
    }

    /// Executes a deterministic slice of bytecode, returning messages to route.
    pub fn run_slice(&mut self, trans: &TransitionBytecode, field_names: &[String], gas_limit: usize) -> Result<Vec<Message>, String> {
        let mut gas_used = 0;
        let mut pc = 0;
        let code = &trans.instructions;
        let consts = &trans.constants;
        let mut outbox = Vec::new();

        while pc < code.len() {
            if gas_used >= gas_limit {
                return Err(format!("Slice exhausted gas boundary (Max {} instructions)", gas_limit));
            }
            gas_used += 1;
            let inst = &code[pc];
            pc += 1;

            match inst {
                OpCode::LoadConst { const_idx, dest_reg } => {
                    match &consts[*const_idx] {
                        Constant::Int(val) => {
                            self.registers[*dest_reg as usize] = *val;
                        }
                        Constant::String(_) => {
                            // Strings unsupported in this basic register yet
                        }
                    }
                }
                OpCode::LoadState { field_idx, dest_reg } => {
                    let key = &field_names[*field_idx];
                    let val = self.state.get(key).unwrap_or(0);
                    self.registers[*dest_reg as usize] = val;
                }
                OpCode::StoreState { src_reg, field_idx } => {
                    let key = &field_names[*field_idx];
                    let val = self.registers[*src_reg as usize];
                    if let Err(e) = self.state.set(key, val) {
                        return Err(e);
                    }
                }
                OpCode::AddInt { r1, r2, dest } => {
                    self.registers[*dest as usize] = self.registers[*r1 as usize] + self.registers[*r2 as usize];
                }
                OpCode::SubInt { r1, r2, dest } => {
                    self.registers[*dest as usize] = self.registers[*r1 as usize] - self.registers[*r2 as usize];
                }
                OpCode::CmpEq { r1, r2, dest } => {
                    self.registers[*dest as usize] = if self.registers[*r1 as usize] == self.registers[*r2 as usize] { 1 } else { 0 };
                }
                OpCode::CmpLt { r1, r2, dest } => {
                    self.registers[*dest as usize] = if self.registers[*r1 as usize] < self.registers[*r2 as usize] { 1 } else { 0 };
                }
                OpCode::CmpGt { r1, r2, dest } => {
                    self.registers[*dest as usize] = if self.registers[*r1 as usize] > self.registers[*r2 as usize] { 1 } else { 0 };
                }
                OpCode::CmpGte { r1, r2, dest } => {
                    self.registers[*dest as usize] = if self.registers[*r1 as usize] >= self.registers[*r2 as usize] { 1 } else { 0 };
                }
                OpCode::CmpLte { r1, r2, dest } => {
                    self.registers[*dest as usize] = if self.registers[*r1 as usize] <= self.registers[*r2 as usize] { 1 } else { 0 };
                }
                OpCode::JumpIfFalse { test_reg, target_offset } => {
                    if self.registers[*test_reg as usize] == 0 {
                        pc = *target_offset;
                    }
                }
                OpCode::Jump { target_offset } => {
                    pc = *target_offset;
                }
                OpCode::SendMsg { target_const_idx, msg_const_idx } => {
                    let target_domain = match &consts[*target_const_idx] {
                        Constant::String(s) => s.clone(),
                        _ => return Err("SendMsg target must be a string".to_string()),
                    };
                    let payload = match &consts[*msg_const_idx] {
                        Constant::String(s) => s.clone(),
                        _ => return Err("SendMsg payload must be a string".to_string()),
                    };
                    outbox.push(Message { target_domain, payload, priority: 0, clock: 0 });
                }
                OpCode::HaltSlice => {
                    break;
                }
            }
        }

        Ok(outbox)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_transition() {
        let fields = vec!["counter".to_string()];
        let mut interp = Interpreter::new(&fields);
        interp.state.set("counter", 10).unwrap();

        let trans = TransitionBytecode {
            name: "test".to_string(),
            constants: vec![Constant::Int(1)],
            instructions: vec![
                OpCode::LoadState { field_idx: 0, dest_reg: 0 },
                OpCode::LoadConst { const_idx: 0, dest_reg: 1 },
                OpCode::AddInt { r1: 0, r2: 1, dest: 2 },
                OpCode::StoreState { src_reg: 2, field_idx: 0 },
                OpCode::HaltSlice,
            ],
        };

        let res = interp.run_slice(&trans, &fields, 1000);
        assert!(res.is_ok());
        assert_eq!(interp.state.get("counter"), Some(11));
    }
}
