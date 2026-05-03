use std::process::Command;

fn main() {
    println!("Building Lexum-cli...");
    let status = Command::new("cargo")
        .arg("build")
        .status()
        .expect("failed to execute process");

    assert!(status.success());
    let mut outputs = Vec::new();
    
    for _i in 0..10 {
        // Remove snapshot
        let _ = std::fs::remove_file("test-files/phase6_test_nondeterminism.Lexum.snapshot.json");
        
        let output = Command::new("cargo")
            .arg("run")
            .arg("-p")
            .arg("Lexum-cli")
            .arg("run")
            .arg("test-files/phase6_test_nondeterminism.Lexum")
            .output()
            .expect("failed to execute process");
            
        let out_str = String::from_utf8_lossy(&output.stdout).to_string();
        // Extract just the scheduler output part
        let scheduler_out = out_str.split("================ SCHEDULER START ================").nth(1).unwrap_or("").to_string();
        outputs.push(scheduler_out);
    }
    
    let first = &outputs[0];
    for (i, out) in outputs.iter().enumerate() {
        if out != first {
            println!("Nondeterminism trace found at run {}!", i);
            println!("EXPECTED:\n{}", first);
            println!("GOT:\n{}", out);
            std::process::exit(1);
        }
    }
    println!("Race condition determinism Confirmed: 10/10 runs identical!");
}
