mod common;
use common::run_ved_program;

#[test]
fn test_message_storm() {
    let program = r#"
        domain Ping {
            state { _x: int }
            transition run {
                slice step {
                    send("Ping", "run")
                    send("Ping", "run")
                }
            }
        }
    "#;

    // This creates an exponentially growing message storm: 1 -> 2 -> 4 -> 8 -> 16
    // The mailbox limits (default 100) will forcefully drop overflow messages.
    // The scheduler natively restricts infinite execution via max_cycles = 1000 bounds.
    let result = run_ved_program(program, vec![("Ping", "run", 0)], 1000);
    
    // We expect it to NOT crash natively. It should hit the 1000 cycles bound natively yielding execution limits securely.
    assert!(!result.converged, "Storm simulation shouldn't quiesce, it should cleanly overflow bounds limits.");
    assert!(result.warning_detected, "Warning should be tripped due to Max Cycles bound hit during storm.");
    assert!(result.steps > 0);
}
