# Ved Runtime

This repository contains the deterministic control-plane programming language prototype, as outlined in the formulation documentation.

## Phase 0 Architecture

- `ved-ir`: Intermediary instructions and definitions
- `ved-compiler`: Lexer, Parser, AST, and Codegen
- `ved-runtime`: execution VM, Mailboxes, and Snapshot loop
- `ved-cli`: command line utility

## Running
Currently stubbed to demonstrate the build environment.

```bash
cargo run -p ved-cli -- run example.vedc
```
