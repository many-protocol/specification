# Gherkin tests

This is a crate to run feature files against a pre-built gherkin
parser to define specification tests for the system.

## Configuring it

Create or edit the `spec.toml` file. It's a toml with two fields:
`server_url` and `faucet_pem`. The `server_url` should point to the
URL of the many server you wish to test. The `faucet_pem` field should
point to an account that has enough balance to run your tests.

## Running it

Build it with `cargo build` and then you can find it under
`target/debug/gherkin-tests`. You need to pass ther `--spec-config`
argument to it, which is the file written above. When you run it,
it'll look for every feature file in your current directory and run
them. You can check `--help` for some additional options.
