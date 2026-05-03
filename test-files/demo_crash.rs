use std::process::Command;
use std::fs;

fn main() {
    let source_file = "../Lexum-examples/public-demo/demo.Lexum";
    let snapshot_file = "../Lexum-examples/public-demo/demo.Lexum.snapshot.json";
    
    // Clean up any old snapshot
    let _ = fs::remove_file(snapshot_file);

    println!("=== 1. Starting First Run (Max 2 Cycles to simulate a crash) ===");
    let mut child = Command::new("cargo")
        .args(&["run", "-p", "Lexum-cli", "--", "run", source_file, "2"])
        .spawn()
        .expect("Failed to run Lexum-cli");
    
    let _ = child.wait();

    println!("\n=== 2. System crashed at cycle 2. Restarting from Snapshot... ===");
    let mut child2 = Command::new("cargo")
        .args(&["run", "-p", "Lexum-cli", "--", "run", source_file, "10"])
        .spawn()
        .expect("Failed to run Lexum-cli");
    
    let _ = child2.wait();
    
    println!("\n=== Demo Complete ===");
}
