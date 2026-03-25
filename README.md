# ved-runtime

Deterministic execution engine for the Ved control-plane programming language.

## Overview

`ved-runtime` implements the core execution model of Ved programs.

It is responsible for:

- deterministic scheduling of transition slices
- persistent state evolution through journal + snapshot mechanisms
- goal-driven reconciliation loops
- effect intent isolation and replayable execution
- crash recovery and logical time progression

The runtime is designed to support long-running orchestration systems that must
continuously stabilize distributed software environments.

## Design Goals

- Predictable and reproducible system behaviour
- Structured isolation between execution domains
- Explicit convergence toward declared goals
- Failure-resilient state persistence
- Minimal trusted core suitable for future distributed evolution

## Status

Early prototype.

Initial milestones achieved:

- Bytecode execution loop
- Append-only journal & snapshots
- Deterministic scheduler and logical clocks
- Basic domain state model
- Developer Experience tools (structured traces, basic compiler errors)

## CLI Usage

The `ved-cli` provides commands to run and inspect Ved programs:

- **Run a program:** `cargo run -p ved-cli -- run path/to/file.ved`
  This compiles the file, runs the scheduler, generates snapshot states (`.snapshot.json`), and creates an execution trace file (`.trace.json`).

- **View a trace:** `cargo run -p ved-cli -- view-trace path/to/file.trace.json`
  Prints a formatted, human-readable view of the exact sequence of domain state changes and messages.

- **Compile only:** `cargo run -p ved-cli -- compile path/to/file.ved`
  Validates syntax and semantics without running the scheduler.

## Repository Structure (planned)

- scheduler
- executor / virtual machine
- persistence engine
- effect runtime
- observability utilities

## Contributing

Design discussions and development are evolving rapidly.
Please open issues before large implementation changes.

## License

Apache License 2.0
