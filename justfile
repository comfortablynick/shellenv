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

# run for (ba)sh
runsh:
    cargo run --release -- -s sh

# run for powershell
runps:
    cargo run --release -- -s ps

# run for fish
runfish:
    cargo run --release -- -s fish

# run for (ba)sh & output to bat
runshc:
    cargo run --release -- -s sh | bat -l sh

# run for powershell & output to bat
runpsc:
    cargo run --release -- -s ps | bat -l powershell

# run for fish & output to bat
runfishc:
    cargo run --release -- -s fish | bat -l fish

test:
    cargo test

fix:
    cargo fix
