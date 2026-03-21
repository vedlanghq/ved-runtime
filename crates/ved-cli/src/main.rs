use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("ved <command>");
        println!("Commands:");
        println!("  compile <file.ved>   - Compile a Ved file to bytecode");
        println!("  run <bundle.vedc>    - Run a Ved bytecode bundle");
        return;
    }

    match args[1].as_str() {
        "run" => {
            if args.len() < 3 {
                println!("Error: Missing bundle file.\nUsage: ved run <bundle.vedc>");
                return;
            }
            let bundle_path = &args[2];
            println!("Starting Ved runtime with bundle: {}", bundle_path);
            ved_runtime::scheduler::run_loop();
        }
        "compile" => {
            if args.len() < 3 {
                println!("Error: Missing source file.\nUsage: ved compile <file.ved>");
                return;
            }
            let source_path = &args[2];
            println!("Compiling Ved source: {}", source_path);
            ved_compiler::compile(source_path);
        }
        _ => println!("Unknown command: {}", args[1]),
    }
}
