mod common;
use common::run_ved_program;

#[test]
fn test_deterministic_execution() {
    let program = r#"
        domain Process {
            state { ticker: int }
            goal done { target ticker >= 10 }
            transition run {
                slice step { ticker = ticker + 1 }
            }
        }
    "#;

    let mut traces = Vec::new();
    for _ in 0..100 {
        let result = run_ved_program(program, vec![("Process", "run", 0)], 100);
        traces.push(result.trace);
    }

    let first_trace = &traces[0];
    for trace in traces.iter().skip(1) {
        assert_eq!(first_trace, trace, "Non-deterministic execution detected in Scheduler runtime trace");
    }
}
