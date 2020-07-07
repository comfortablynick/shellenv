#!/usr/bin/env just --justfile
bin_name := 'shellenv'
dev := '1'

alias r := run
alias b := build
alias i := install
alias h := help

# build release binary
build:
    cargo build --release

# build release binary ONLY during dev; otherwise install
install:
    #!/usr/bin/env bash
    if [[ ${DEV:-{{dev}}} -eq "1" ]]; then
        cargo run --release
    else
        cargo install -f --path .
    fi #

# build release binary and run
run +args='':
    cargo run --release -- {{args}}

help:
    ./target/release/{{bin_name}} -h

# run release binary
rb +args='':
    ./target/release/{{bin_name}} {{args}}

runsh:
    cargo run --release -- -s sh | bat -l sh

runps:
    cargo run --release -- -s ps | bat -l powershell

runfish:
    cargo run --release -- -s ps | bat -l fish

test:
    cargo test

fix:
    cargo fix
