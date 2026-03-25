use std::env;

use ved_runtime::domain_registry::{DomainInstance, DomainRegistry};
use ved_runtime::scheduler::Scheduler;
use ved_runtime::messaging::Message;
use ved_runtime::persistence::SnapshotManager;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("ved <command>");
        println!("Commands:");
        println!("  compile <file.ved>       - Compile a Ved file to bytecode");
        println!("  run <file.ved>           - Run a Ved file directly");
        println!("  view-trace <trace.json>  - View an execution trace");
        return;
    }

    match args[1].as_str() {
        "view-trace" => {
            if args.len() < 3 {
                println!("Error: Missing trace file.\nUsage: ved view-trace <file.trace.json>");
                return;
            }
            let trace_path = &args[2];
            println!("[CLI] Loading execution trace: {}", trace_path);
            let content = std::fs::read_to_string(trace_path).unwrap_or_else(|e| {
                println!("Error reading trace file: {}", e);
                std::process::exit(1);
            });
            
            match ved_tracer::Tracer::format_trace_from_json(&content) {
                Ok(lines) => {
                    println!("\n--- EXECUTION TRACE VIEW ---");
                    for line in lines {
                        println!("{}", line);
                    }
                    println!("----------------------------");
                }
                Err(e) => {
                    println!("Failed to parse trace JSON: {}", e);
                }
            }
        }
        "run" => {
            if args.len() < 3 {
                println!("Error: Missing source file.\nUsage: ved run <file.ved>");
                return;
            }
            let source_path = &args[2];
            println!("[CLI] Reading source: {}", source_path);
            let source = std::fs::read_to_string(source_path).unwrap_or_else(|e| {
                println!("Error reading {}: {}", source_path, e);
                std::process::exit(1);
            });

            println!("[CLI] Compiling...");
            match ved_compiler::compile_source(&source) {
                Ok(program) => {
                    println!("[CLI] Compilation successful. Initiating Runtime.");
                    let mut registry = DomainRegistry::new();

                    for domain in program.domains {
                        println!("[Runtime] Initializing Domain: {}", domain.name);
                        let instance = DomainInstance::new(
                            domain.name.clone(),
                            domain.state_schema.clone(),
                            domain.clone(),
                        );
                        registry.register(instance);
                    }

                    let snapshot_file = format!("{}.snapshot.json", source_path);
                    let snapshot_mgr = SnapshotManager::new(&snapshot_file);
                    let mut is_resumed = false;

                    match snapshot_mgr.load() {
                        Ok(data) => {
                            println!("[CLI] Resuming from snapshot (cycle {})...", data.cycle);
                            if let Err(e) = snapshot_mgr.restore_into(data, &mut registry) {
                                println!("[CLI] Critical Error restoring snapshot: {}", e);
                                std::process::exit(1);
                            }
                            is_resumed = true;
                        }
                        Err(e) => {
                            println!("[CLI] No valid snapshot found ({}). Starting fresh.", e);
                        }
                    }

                    if !is_resumed {
                        let start_domain = if registry.instances.contains_key("Producer") {
                            "Producer".to_string()
                        } else if let Some(first_domain) = {
                            // Sort keys deterministically
                            let mut keys: Vec<&String> = registry.instances.keys().collect();
                            keys.sort();
                            keys.first().map(|k| k.to_string())
                        } {
                            first_domain
                        } else {
                            println!("[CLI] No domains loaded.");
                            return;
                        };

                        let first_trans = registry.instances.get(&start_domain).unwrap().bytecode.transitions.first();
                        let default_trans_name = if let Some(trans) = first_trans { trans.name.clone() } else { "run".to_string() };

                        let boot_msg = Message {
                            target_domain: start_domain.to_string(),
                            payload: default_trans_name,
                            priority: 0,
                            clock: 0,
                        };

                        println!("[CLI] Seeding boot message: {:?}", boot_msg);
                        let _ = registry.route_message(boot_msg);
                    }

                    let mut scheduler = Scheduler::new(registry).with_snapshots(snapshot_mgr);
                    println!("\n================ SCHEDULER START ================");
                    
                    let mut max_cycles = 100;
                    if args.len() > 3 {
                        if let Ok(c) = args[3].parse::<usize>() {
                            max_cycles = c;
                        }
                    }

                    let trace = scheduler.execute_until_quiescent(max_cycles, 1000);
                    for line in trace {
                        println!("{}", line);
                    }
                    
                    let trace_file = format!("{}.trace.json", source_path);
                    let json_trace = scheduler.tracer.dump_json();
                    if let Err(e) = std::fs::write(&trace_file, json_trace) {
                        println!("[CLI] Error writing trace file: {}", e);
                    } else {
                        println!("[CLI] Wrote execution trace to {}", trace_file);
                    }

                    println!("================ SCHEDULER HALT ================\n");
                    println!("[CLI] Execution complete. Quiescence reached.");
                }
                Err(e) => {
                    println!("Error during compilation:\n{}", e);
                }
            }
        }
        "compile" => {
            if args.len() < 3 {
                println!("Error: Missing source file.\nUsage: ved compile <file.ved>");
                return;
            }
            let source_path = &args[2];
            let source = std::fs::read_to_string(source_path).unwrap();
            match ved_compiler::compile_source(&source) {
                Ok(_program) => println!("Compilation successful."),
                Err(e) => println!("Error during compilation:\n{}", e),
            }
        }
        _ => println!("Unknown command: {}", args[1]),
    }
}
