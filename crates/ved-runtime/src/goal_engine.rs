use crate::interpreter::Interpreter;
use ved_ir::bytecode::{GoalBytecode, OpCode};
use crate::state::IsolatedState;

pub struct GoalEngine;

impl GoalEngine {
    /// Evaluates if a given goal is met by executing its bytecode predicate against a snapshot of memory.
    pub fn evaluate(goal: &GoalBytecode, state: &IsolatedState, schema: &[String], gas_limit: usize) -> Result<bool, String> {
        let mut interp = Interpreter::with_state(state.snapshot());
        
        let mut gas_used = 0;
        let mut pc = 0;
        let code = &goal.instructions;
        let consts = &goal.constants;
        
        // Track the last register written to return it as the predicate status
        let mut last_written_reg: Option<u8> = None;

        while pc < code.len() {
            if gas_used >= gas_limit {
                return Err(format!("Goal evaluation exhausted gas boundary (Max {} instructions)", gas_limit));
            }
            gas_used += 1;
            let inst = &code[pc];
            pc += 1;

            match inst {
                OpCode::LoadConst { const_idx, dest_reg } => {
                    match &consts[*const_idx] {
                        ved_ir::bytecode::Constant::Int(val) => {
                            interp.registers[*dest_reg as usize] = *val;
                            last_written_reg = Some(*dest_reg);
                        }
                        ved_ir::bytecode::Constant::String(_) => { } // Unsupported in this basic version for eval
                    }
                }
                OpCode::LoadState { field_idx, dest_reg } => {
                    let key = &schema[*field_idx];
                    let val = interp.state.get(key).unwrap_or(0);
                    interp.registers[*dest_reg as usize] = val;
                    last_written_reg = Some(*dest_reg);
                }
                OpCode::CmpEq { r1, r2, dest } => {
                    interp.registers[*dest as usize] = if interp.registers[*r1 as usize] == interp.registers[*r2 as usize] { 1 } else { 0 };
                    last_written_reg = Some(*dest);
                }
                OpCode::CmpLt { r1, r2, dest } => {
                    interp.registers[*dest as usize] = if interp.registers[*r1 as usize] < interp.registers[*r2 as usize] { 1 } else { 0 };
                    last_written_reg = Some(*dest);
                }
                OpCode::CmpGt { r1, r2, dest } => {
                    interp.registers[*dest as usize] = if interp.registers[*r1 as usize] > interp.registers[*r2 as usize] { 1 } else { 0 };
                    last_written_reg = Some(*dest);
                }
                OpCode::CmpGte { r1, r2, dest } => {
                    interp.registers[*dest as usize] = if interp.registers[*r1 as usize] >= interp.registers[*r2 as usize] { 1 } else { 0 };
                    last_written_reg = Some(*dest);
                }
                OpCode::CmpLte { r1, r2, dest } => {
                    interp.registers[*dest as usize] = if interp.registers[*r1 as usize] <= interp.registers[*r2 as usize] { 1 } else { 0 };
                    last_written_reg = Some(*dest);
                }
                OpCode::HaltSlice => break,
                // We prevent goals from mutating state or sending messages
                OpCode::StoreState { .. } | OpCode::SendMsg { .. } => {
                    return Err("Illegal operation in Goal context: Side effects are forbidden.".into());
                }
                OpCode::AddInt { r1, r2, dest } => {
                    interp.registers[*dest as usize] = interp.registers[*r1 as usize] + interp.registers[*r2 as usize];
                    last_written_reg = Some(*dest);
                }
                OpCode::SubInt { r1, r2, dest } => {
                    interp.registers[*dest as usize] = interp.registers[*r1 as usize] - interp.registers[*r2 as usize];
                    last_written_reg = Some(*dest);
                }
                OpCode::MulInt { r1, r2, dest } => {
                    interp.registers[*dest as usize] = interp.registers[*r1 as usize] * interp.registers[*r2 as usize];
                    last_written_reg = Some(*dest);
                }
                OpCode::DivInt { r1, r2, dest } => {
                    let divisor = interp.registers[*r2 as usize];
                    if divisor == 0 { return Err("Division by zero in Goal context".into()); }
                    interp.registers[*dest as usize] = interp.registers[*r1 as usize] / divisor;
                    last_written_reg = Some(*dest);
                }
                _ => { /* evaluate other ops */ }
            }
        }

        if let Some(reg) = last_written_reg {
            Ok(interp.registers[reg as usize] != 0)
        } else {
            Err("Goal evaluated to no resulting value.".into())
        }
    }

    pub fn evaluate_invariant(invariant: &ved_ir::bytecode::InvariantBytecode, state: &IsolatedState, schema: &[String], gas_limit: usize) -> Result<bool, String> {
        let mut interp = Interpreter::with_state(state.snapshot());
        
        let mut gas_used = 0;
        let mut pc = 0;
        let code = &invariant.instructions;
        let consts = &invariant.constants;
        
        let mut last_written_reg: Option<u8> = None;

        while pc < code.len() {
            if gas_used >= gas_limit {
                return Err(format!("Invariant evaluation exhausted gas boundary (Max {} instructions)", gas_limit));
            }
            gas_used += 1;
            let inst = &code[pc];
            pc += 1;

            match inst {
                OpCode::LoadConst { const_idx, dest_reg } => {
                    match &consts[*const_idx] {
                        ved_ir::bytecode::Constant::Int(val) => {
                            interp.registers[*dest_reg as usize] = *val;
                            last_written_reg = Some(*dest_reg);
                        }
                        ved_ir::bytecode::Constant::String(_) => { }
                    }
                }
                OpCode::LoadState { field_idx, dest_reg } => {
                    let key = &schema[*field_idx];
                    let val = interp.state.get(key).unwrap_or(0);
                    interp.registers[*dest_reg as usize] = val;
                    last_written_reg = Some(*dest_reg);
                }
                OpCode::CmpEq { r1, r2, dest } => {
                    interp.registers[*dest as usize] = if interp.registers[*r1 as usize] == interp.registers[*r2 as usize] { 1 } else { 0 };
                    last_written_reg = Some(*dest);
                }
                OpCode::CmpLt { r1, r2, dest } => {
                    interp.registers[*dest as usize] = if interp.registers[*r1 as usize] < interp.registers[*r2 as usize] { 1 } else { 0 };
                    last_written_reg = Some(*dest);
                }
                OpCode::CmpGt { r1, r2, dest } => {
                    interp.registers[*dest as usize] = if interp.registers[*r1 as usize] > interp.registers[*r2 as usize] { 1 } else { 0 };
                    last_written_reg = Some(*dest);
                }
                OpCode::CmpGte { r1, r2, dest } => {
                    interp.registers[*dest as usize] = if interp.registers[*r1 as usize] >= interp.registers[*r2 as usize] { 1 } else { 0 };
                    last_written_reg = Some(*dest);
                }
                OpCode::CmpLte { r1, r2, dest } => {
                    interp.registers[*dest as usize] = if interp.registers[*r1 as usize] <= interp.registers[*r2 as usize] { 1 } else { 0 };
                    last_written_reg = Some(*dest);
                }
                OpCode::HaltSlice => break,
                OpCode::StoreState { .. } | OpCode::SendMsg { .. } => {
                    return Err("Illegal operation in Invariant context: Side effects are forbidden.".into());
                }
                OpCode::AddInt { r1, r2, dest } => {
                    interp.registers[*dest as usize] = interp.registers[*r1 as usize] + interp.registers[*r2 as usize];
                    last_written_reg = Some(*dest);
                }
                OpCode::SubInt { r1, r2, dest } => {
                    interp.registers[*dest as usize] = interp.registers[*r1 as usize] - interp.registers[*r2 as usize];
                    last_written_reg = Some(*dest);
                }
                OpCode::MulInt { r1, r2, dest } => {
                    interp.registers[*dest as usize] = interp.registers[*r1 as usize] * interp.registers[*r2 as usize];
                    last_written_reg = Some(*dest);
                }
                OpCode::DivInt { r1, r2, dest } => {
                    let divisor = interp.registers[*r2 as usize];
                    if divisor == 0 { return Err("Division by zero in Invariant context".into()); }
                    interp.registers[*dest as usize] = interp.registers[*r1 as usize] / divisor;
                    last_written_reg = Some(*dest);
                }
                _ => {}
            }
        }

        if let Some(reg) = last_written_reg {
            Ok(interp.registers[reg as usize] != 0)
        } else {
            Err("Invariant evaluated to no resulting value.".into())
        }
    }
}
