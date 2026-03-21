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

Initial milestones include:

- bytecode execution loop
- append-only journal
- deterministic scheduler skeleton
- basic domain state model

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
