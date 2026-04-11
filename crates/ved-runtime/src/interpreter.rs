use ved_ir::bytecode::{OpCode, Constant, TransitionBytecode};
use crate::messaging::Message;
use crate::state::IsolatedState;

pub struct Interpreter {
    pub state: IsolatedState,
    pub registers: [i64; 256],
    pub available_capabilities: Vec<String>,
}

pub enum SliceResult {
    Completed(Vec<Message>),
    Suspended { pc: usize, outbox: Vec<Message> },
    Fault(String),
}

impl Interpreter {
    pub fn new(schema: &[String]) -> Self {
        Self {
            state: IsolatedState::new(schema),
            registers: [0; 256],
            available_capabilities: vec![],
        }
    }

    pub fn with_state(state: IsolatedState) -> Self {
        Self {
            state,
            registers: [0; 256],
            available_capabilities: vec![],
        }
    }

    pub fn set_capabilities(&mut self, caps: Vec<String>) {
        self.available_capabilities = caps;
    }

    /// Executes a deterministic slice of bytecode, returning messages to route.
    pub fn run_slice(&mut self, trans: &TransitionBytecode, field_names: &[String], gas_limit: usize, start_pc: usize, mut outbox: Vec<Message>) -> SliceResult {
        let mut gas_used = 0;
        let mut pc = start_pc;
        let code = &trans.instructions;
        let consts = &trans.constants;

        while pc < code.len() {
            if gas_used >= gas_limit {
                return SliceResult::Suspended { pc, outbox };
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
                        return SliceResult::Fault(e);
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
                OpCode::MulInt { r1, r2, dest } => {
                    self.registers[*dest as usize] = self.registers[*r1 as usize] * self.registers[*r2 as usize];
                }
                OpCode::DivInt { r1, r2, dest } => {
                    let divisor = self.registers[*r2 as usize];
                    if divisor == 0 {
                        return SliceResult::Fault("Deterministic fault: Division by zero".to_string());
                    }
                    self.registers[*dest as usize] = self.registers[*r1 as usize] / divisor;
                }
                OpCode::AndBool { r1, r2, dest } => {
                    let a = self.registers[*r1 as usize] != 0;
                    let b = self.registers[*r2 as usize] != 0;
                    self.registers[*dest as usize] = if a && b { 1 } else { 0 };
                }
                OpCode::OrBool { r1, r2, dest } => {
                    let a = self.registers[*r1 as usize] != 0;
                    let b = self.registers[*r2 as usize] != 0;
                    self.registers[*dest as usize] = if a || b { 1 } else { 0 };
                }
                OpCode::NotBool { r1, dest } => {
                    let a = self.registers[*r1 as usize] != 0;
                    self.registers[*dest as usize] = if !a { 1 } else { 0 };
                }
                OpCode::ListLen { .. } | OpCode::ListGet { .. } | OpCode::ListAppend { .. } => {
                    return SliceResult::Fault("Deterministic fault: List commands unsupported in Phase 1 runtime".to_string());
                }
                OpCode::EmitEffect { .. } | OpCode::CheckGoal { .. } => {
                    return SliceResult::Fault("Deterministic fault: Effects and Advanced Goal checking via bytecode not yet wired".to_string());
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
                        _ => return SliceResult::Fault("SendMsg target must be a string".to_string()),
                    };
                    
                    if !self.available_capabilities.contains(&format!("send_to:{}", target_domain)) && !self.available_capabilities.contains(&"root".to_string()) {
                        return SliceResult::Fault(format!("Authority/Scope breach: missing capability 'send_to:{}'", target_domain));
                    }
                    
                    let payload = match &consts[*msg_const_idx] {
                        Constant::String(s) => s.clone(),
                        _ => return SliceResult::Fault("SendMsg payload must be a string".to_string()),
                    };
                    outbox.push(Message { id: "".to_string(), source_domain: "".to_string(), target_domain, payload, priority: 0, clock: 0 });
                }
                OpCode::SendHighMsg { target_const_idx, msg_const_idx } => {
                    let target_domain = match &consts[*target_const_idx] {
                        Constant::String(s) => s.clone(),
                        _ => return SliceResult::Fault("SendHighMsg target must be a string".to_string()),
                    };
                    
                    if !self.available_capabilities.contains(&format!("send_to:{}", target_domain)) && !self.available_capabilities.contains(&"root".to_string()) {
                        return SliceResult::Fault(format!("Authority/Scope breach: missing capability 'send_to:{}'", target_domain));
                    }
                    
                    let payload = match &consts[*msg_const_idx] {
                        Constant::String(s) => s.clone(),
                        _ => return SliceResult::Fault("SendHighMsg payload must be a string".to_string()),
                    };
                    outbox.push(Message { id: "".to_string(), source_domain: "".to_string(), target_domain, payload, priority: 1, clock: 0 });
                }
                OpCode::HaltSlice => {
                    break;
                }
            }
        }

        SliceResult::Completed(outbox)
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
            required_capabilities: vec![],
            scope: None,
            constants: vec![Constant::Int(1)],
            instructions: vec![
                OpCode::LoadState { field_idx: 0, dest_reg: 0 },
                OpCode::LoadConst { const_idx: 0, dest_reg: 1 },
                OpCode::AddInt { r1: 0, r2: 1, dest: 2 },
                OpCode::StoreState { src_reg: 2, field_idx: 0 },
                OpCode::HaltSlice,
            ],
        };

        let res = interp.run_slice(&trans, &fields, 1000, 0, vec![]);
        assert!(matches!(res, SliceResult::Completed(_)));
        assert_eq!(interp.state.get("counter"), Some(11));
    }
}
