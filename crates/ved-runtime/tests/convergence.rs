mod common;
use common::run_ved_program;

#[test]
fn test_convergence() {
    let program = r#"
        domain Target {
            state { _x: int }
            transition stabilize {
                slice step { 
                    _x = 0
                }
            }
        }
    "#;

    let result = run_ved_program(program, vec![("Target", "stabilize", 0)], 200);
    
    assert!(result.converged, "System failed to converge onto structural targets securely.");
    assert!(!result.warning_detected, "Baseline convergence should not trip heuristic maximum bounds.");
}

#[test]
fn test_oscillation_detection() {
    let program = r#"
        domain Target {
            capability { messaging, send_to:Target }
            state { _x: int }
            transition oscillate {
                capability { messaging, send_to:Target }
                slice step {
                    send("Target", "oscillate")
                }
            }
        }
    "#;

    let result = run_ved_program(program, vec![("Target", "oscillate", 0)], 50);
    
    assert!(!result.converged, "Infinite recursion logic should actively prevent system quiescence natively.");
    assert!(result.warning_detected, "Linear recursion detection missed failure oscillation.");
}
