# Gherkin tests

This is a crate to run feature files against a pre-built gherkin
parser to define specification tests for the system.

## Running it

Build it with `cargo build` and then you can find it under
`target/debug/gherkin-tests`. When you run it, it'll look for every
feature file in your current directory and run them. You can check
`--help` for some additional options.
