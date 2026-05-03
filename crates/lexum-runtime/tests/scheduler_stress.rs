mod common;
use common::run_ved_program;

#[test]
fn test_no_starvation() {
    let program = r#"
        domain HighPriority {
            capability { messaging, send_to:HighPriority }
            state { counter: int }
            transition Spam {
                capability { messaging, send_to:HighPriority }
                slice step {
                    send_high("HighPriority", "Spam")
                }
            }
        }
        
        domain LowPriority {
            state { executed: int }
            transition RunOnce {
                slice step {
                    executed = 1
                }
            }
        }
    "#;

    let result = run_ved_program(
        program,
        vec![
            ("HighPriority", "Spam", 1),
            ("LowPriority", "RunOnce", 0),
        ],
        20,
    );
    
    // We expect the scheduler to process High Priority spam, but ensure starvation 
    // controls eventually schedule the low priority ticket natively before max cycles hit.
    assert!(result.low_priority_executed, "Low priority execution starvation detected across execution boundary.");
}
