use ved_ir::bytecode::{TransitionBytecode, OpCode, Constant};
use ved_runtime::interpreter::{Interpreter, SliceResult};
use ved_runtime::messaging::{Message, Mailbox};
use ved_runtime::state::IsolatedState;

#[test]
fn test_1c_instruction_budgeting() {
    // 1.C Instruction budgeting correctness - "infinite loop inside single slice"
    let schema = vec![];
    let state = IsolatedState::new(&schema);
    let mut interpreter = Interpreter::with_state(state.snapshot());
    
    // Create bytecode with an infinite loop: Jump to 0.
    let trans = TransitionBytecode {
        name: "infinite_loop".to_string(),
        scope: None,
        required_capabilities: vec![],
        instructions: vec![
            OpCode::LoadConst { const_idx: 0, dest_reg: 0 },
            OpCode::Jump { target_offset: 0 }, // infinite loop
        ],
        constants: vec![Constant::Int(42)],
    };

    let field_names = vec![];
    
    // Set gas limit very low to test exhaustion
    let gas_limit = 50;
    
    let result = interpreter.run_slice(&trans, &field_names, gas_limit, 0, vec![]);
    
    match result {
        SliceResult::Suspended { .. } => {
            // Expected! Budget exhausted.
        },
        SliceResult::Fault(msg) => panic!("Expected structural suspension yielding, not hard fault! {}", msg),
        SliceResult::Completed(_) => panic!("Expected slice to exhaust instruction budget and suspend!"),
    }
}

#[test]
fn test_3b_starvation_edge_cases() {
    // 3.B Starvation edge cases - "Ensure bounded progress against infinite high-priority producer"
    let mut mailbox = Mailbox::new(100);
    
    // Push 5 high-priority messages
    for i in 0..5 {
        mailbox.push(Message { id: "".into(), source_domain: "SYSTEM".into(),
            target_domain: "Target".to_string(),
            payload: format!("high_{}", i),
            priority: 1,
            clock: 0,
        }).unwrap();
    }
    
    // Push 5 normal messages
    for i in 0..5 {
        mailbox.push(Message { id: "".into(), source_domain: "SYSTEM".into(),
            target_domain: "Target".to_string(),
            payload: format!("normal_{}", i),
            priority: 0,
            clock: 0,
        }).unwrap();
    }
    
    // With 3-skip fairness rule, we should see:
    // Pop 1: High 0
    // Pop 2: High 1
    // Pop 3: High 2
    // Pop 4: Normal 0 (Starvation prevention kick-in!)
    // Pop 5: High 3
    // Pop 6: High 4
    // Pop 7: Normal 1
    
    assert_eq!(mailbox.pop().unwrap().payload, "high_0");
    assert_eq!(mailbox.pop().unwrap().payload, "high_1");
    assert_eq!(mailbox.pop().unwrap().payload, "high_2");
    
    // STARVATION PREVENTION: After 3 consecutive highs, normal is forced!
    let forced_normal = mailbox.pop().unwrap();
    assert_eq!(forced_normal.payload, "normal_0", "Starvation logic failed: expected normal message round-robin.");
    
    // Back to high
    assert_eq!(mailbox.pop().unwrap().payload, "high_3");
    assert_eq!(mailbox.pop().unwrap().payload, "high_4");
    
    // Rest of normal
    assert_eq!(mailbox.pop().unwrap().payload, "normal_1");
}
