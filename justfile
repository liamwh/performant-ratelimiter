#!/usr/bin/env -S just --justfile
export CARGO_TERM_COLOR := "always"

# Show available commands
default:
    @just --list --justfile {{justfile()}}


# Run the benchmarks, and create the report using criterion
bench:
    cargo bench

# Run the benchmarks, and produce a flamegraph using pprof
profile:
    cargo bench --bench ratelimit_benchmark -- --profile-time=45
