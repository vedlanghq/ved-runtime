
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Int(i64),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    // VM Control & State
    LoadConst { const_idx: usize, dest_reg: u8 },
    LoadState { field_idx: usize, dest_reg: u8 },
    StoreState { src_reg: u8, field_idx: usize },
    
    // Arithmetic
    AddInt { r1: u8, r2: u8, dest: u8 },
    SubInt { r1: u8, r2: u8, dest: u8 },
    MulInt { r1: u8, r2: u8, dest: u8 },
    DivInt { r1: u8, r2: u8, dest: u8 },
    
    // Comparisons
    CmpEq { r1: u8, r2: u8, dest: u8 },
    CmpLt { r1: u8, r2: u8, dest: u8 },
    CmpGt { r1: u8, r2: u8, dest: u8 },
    CmpGte { r1: u8, r2: u8, dest: u8 },
    CmpLte { r1: u8, r2: u8, dest: u8 },
    
    // Logical
    AndBool { r1: u8, r2: u8, dest: u8 },
    OrBool { r1: u8, r2: u8, dest: u8 },
    NotBool { r1: u8, dest: u8 },
    
    // Control Flow
    JumpIfFalse { test_reg: u8, target_offset: usize },
    Jump { target_offset: usize },
    
    // Lists
    ListLen { target_reg: u8, dest_reg: u8 },
    ListGet { target_reg: u8, idx_reg: u8, dest_reg: u8 },
    ListAppend { target_reg: u8, val_reg: u8 },
    
    // Messaging & IO
    SendMsg { target_const_idx: usize, msg_const_idx: usize },
    SendHighMsg { target_const_idx: usize, msg_const_idx: usize },
    EmitEffect { effect_idx: usize, arg_regs: Vec<u8> },
    CheckGoal { goal_idx: usize },
    
    HaltSlice,
}

#[derive(Debug, Clone)]
pub struct TransitionBytecode {
    pub name: String,
    pub scope: Option<String>,
    pub required_capabilities: Vec<String>,
    pub constants: Vec<Constant>,
    pub instructions: Vec<OpCode>,
}

#[derive(Debug, Clone)]
pub struct GoalBytecode {
    pub name: String,
    pub scope: Option<String>,
    pub required_capabilities: Vec<String>,
    pub priority: u8,
    pub recovery_transitions: Vec<String>,
    pub constants: Vec<Constant>,
    pub instructions: Vec<OpCode>,
}

#[derive(Debug, Clone)]
pub struct InvariantBytecode {
    pub name: String,
    pub constants: Vec<Constant>,
    pub instructions: Vec<OpCode>,
}

#[derive(Debug, Clone)]
pub struct DomainBytecode {
    pub name: String,
    pub scope: Option<String>,
    pub capability_manifest: Vec<String>,
    pub state_schema: Vec<String>,
    pub transitions: Vec<TransitionBytecode>,
    pub goals: Vec<GoalBytecode>,
    pub invariants: Vec<InvariantBytecode>,
}

#[derive(Debug, Clone)]
pub struct BytecodeProgram {
    pub domains: Vec<DomainBytecode>,
}

// ==== CUSTOM BINARY PACKER ====

impl OpCode {
    fn opcode_tag(&self) -> u8 {
        match self {
            OpCode::LoadConst { .. } => 0x01,
            OpCode::LoadState { .. } => 0x02,
            OpCode::StoreState { .. } => 0x03,
            
            OpCode::AddInt { .. } => 0x0A,
            OpCode::SubInt { .. } => 0x0B,
            OpCode::MulInt { .. } => 0x0C,
            OpCode::DivInt { .. } => 0x0D,
            
            OpCode::CmpEq { .. } => 0x10,
            OpCode::CmpLt { .. } => 0x11,
            OpCode::CmpGt { .. } => 0x12,
            OpCode::CmpGte { .. } => 0x13,
            OpCode::CmpLte { .. } => 0x14,
            
            OpCode::AndBool { .. } => 0x1A,
            OpCode::OrBool { .. } => 0x1B,
            OpCode::NotBool { .. } => 0x1C,
            
            OpCode::JumpIfFalse { .. } => 0x20,
            OpCode::Jump { .. } => 0x21,
            
            OpCode::ListLen { .. } => 0x30,
            OpCode::ListGet { .. } => 0x31,
            OpCode::ListAppend { .. } => 0x32,
            
            OpCode::SendMsg { .. } => 0x40,
            OpCode::SendHighMsg { .. } => 0x41,
            OpCode::EmitEffect { .. } => 0x42,
            OpCode::CheckGoal { .. } => 0x43,
            
            OpCode::HaltSlice => 0xFF,
        }
    }

    /// Packs instruction into [opcode: u8] [operand_count: u8] [operands: bytes...]
    pub fn pack(&self, buf: &mut Vec<u8>) {
        buf.push(self.opcode_tag());
        
        // Operand buffer
        let mut ops = Vec::new();
        let mut op_count = 0;

        macro_rules! pack_u8 { ($v:expr) => { { ops.push(*$v); op_count += 1; } } }
        macro_rules! pack_usize { ($v:expr) => { { ops.extend_from_slice(&(*$v as u32).to_le_bytes()); op_count += 1; } } }

        match self {
            OpCode::LoadConst { const_idx, dest_reg } => { pack_usize!(const_idx); pack_u8!(dest_reg); }
            OpCode::LoadState { field_idx, dest_reg } => { pack_usize!(field_idx); pack_u8!(dest_reg); }
            OpCode::StoreState { src_reg, field_idx } => { pack_u8!(src_reg); pack_usize!(field_idx); }
            OpCode::AddInt { r1, r2, dest } | OpCode::SubInt { r1, r2, dest } | OpCode::MulInt { r1, r2, dest } | OpCode::DivInt { r1, r2, dest } |
            OpCode::CmpEq { r1, r2, dest } | OpCode::CmpLt { r1, r2, dest } | OpCode::CmpGt { r1, r2, dest } | OpCode::CmpGte { r1, r2, dest } | OpCode::CmpLte { r1, r2, dest } |
            OpCode::AndBool { r1, r2, dest } | OpCode::OrBool { r1, r2, dest } |
            OpCode::ListGet { target_reg: r1, idx_reg: r2, dest_reg: dest } => {
                pack_u8!(r1); pack_u8!(r2); pack_u8!(dest); 
            }
            OpCode::NotBool { r1, dest } | OpCode::ListLen { target_reg: r1, dest_reg: dest } | OpCode::ListAppend { target_reg: r1, val_reg: dest } => {
                pack_u8!(r1); pack_u8!(dest);
            }
            OpCode::JumpIfFalse { test_reg, target_offset } => { pack_u8!(test_reg); pack_usize!(target_offset); }
            OpCode::Jump { target_offset } => { pack_usize!(target_offset); }
            OpCode::SendMsg { target_const_idx, msg_const_idx } | OpCode::SendHighMsg { target_const_idx, msg_const_idx } => {
                pack_usize!(target_const_idx); pack_usize!(msg_const_idx);
            }
            OpCode::EmitEffect { effect_idx, arg_regs } => {
                pack_usize!(effect_idx);
                // dynamic arguments length; handled as a single struct operand conceptually, but we track length
                ops.push(arg_regs.len() as u8);
                ops.extend_from_slice(arg_regs);
                op_count += 2; // the arg count itself + the packed array
            }
            OpCode::CheckGoal { goal_idx } => { pack_usize!(goal_idx); }
            OpCode::HaltSlice => {}
        }
        
        buf.push(op_count);
        buf.extend_from_slice(&ops);
    }
}

pub struct BinaryPacker;

impl BinaryPacker {
    fn write_string(s: &str, buf: &mut Vec<u8>) {
        let bytes = s.as_bytes();
        buf.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(bytes);
    }

    fn write_constants(consts: &[Constant], buf: &mut Vec<u8>) {
        buf.extend_from_slice(&(consts.len() as u16).to_le_bytes());
        for c in consts {
            match c {
                Constant::Int(v) => {
                    buf.push(0);
                    buf.extend_from_slice(&v.to_le_bytes());
                }
                Constant::String(s) => {
                    buf.push(1);
                    Self::write_string(s, buf);
                }
            }
        }
    }

    pub fn serialize(prog: &BytecodeProgram) -> Vec<u8> {
        let mut buf = Vec::new();
        // Magic header VEDC
        buf.extend_from_slice(b"VEDC");
        
        // Version
        buf.push(1);
        
        // Domains count
        buf.extend_from_slice(&(prog.domains.len() as u16).to_le_bytes());
        
        for dom in &prog.domains {
            Self::write_string(&dom.name, &mut buf);
            
            // Schema
            buf.extend_from_slice(&(dom.state_schema.len() as u16).to_le_bytes());
            for field in &dom.state_schema {
                Self::write_string(field, &mut buf);
            }
            
            // Transitions
            buf.extend_from_slice(&(dom.transitions.len() as u16).to_le_bytes());
            for tr in &dom.transitions {
                Self::write_string(&tr.name, &mut buf);
                Self::write_constants(&tr.constants, &mut buf);
                
                // Instructions
                let mut instr_buf = Vec::new();
                for instr in &tr.instructions {
                    instr.pack(&mut instr_buf);
                }
                buf.extend_from_slice(&(instr_buf.len() as u32).to_le_bytes());
                buf.extend_from_slice(&instr_buf);
            }
            
            // Goals
            buf.extend_from_slice(&(dom.goals.len() as u16).to_le_bytes());
            for g in &dom.goals {
                Self::write_string(&g.name, &mut buf);
                Self::write_constants(&g.constants, &mut buf);
                
                buf.extend_from_slice(&(g.recovery_transitions.len() as u16).to_le_bytes());
                for rt in &g.recovery_transitions {
                    Self::write_string(rt, &mut buf);
                }
                
                let mut instr_buf = Vec::new();
                for instr in &g.instructions {
                    instr.pack(&mut instr_buf);
                }
                buf.extend_from_slice(&(instr_buf.len() as u32).to_le_bytes());
                buf.extend_from_slice(&instr_buf);
            }
            
            // Invariants
            buf.extend_from_slice(&(dom.invariants.len() as u16).to_le_bytes());
            for inv in &dom.invariants {
                Self::write_string(&inv.name, &mut buf);
                Self::write_constants(&inv.constants, &mut buf);
                
                let mut instr_buf = Vec::new();
                for instr in &inv.instructions {
                    instr.pack(&mut instr_buf);
                }
                buf.extend_from_slice(&(instr_buf.len() as u32).to_le_bytes());
                buf.extend_from_slice(&instr_buf);
            }
        }
        buf
    }
}
