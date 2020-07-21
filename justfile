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

# rebuild docs
doc:
    cargo doc --release

# start server for docs and update upon changes
docslive:
    light-server -c .lightrc

# docslive +PORT='40000':
#     cargo watch -x 'doc --release --color=always' -s 'live-server target/doc --no-browser --port={{PORT}}'

# rebuild docs and start simple static server
docs +PORT='40000':
    cargo watch -x 'doc --release' -s 'http target/doc -p {{PORT}}'

# build release binary and run
run +args='':
    cargo run --release -- {{args}}

help:
    ./target/release/{{bin_name}} -h

# run release binary
rb +args='':
    ./target/release/{{bin_name}} {{args}}

# run for (ba)sh
runsh +args='':
    cargo run --release -- -s sh {{args}}

# run for zsh
runzsh +args='':
    cargo run --release -- -s zsh {{args}}

# run for powershell
runps +args='':
    cargo run --release -- -s ps {{args}}

# run for fish
runfish +args='':
    cargo run --release -- -s fish {{args}}

# run for (ba)sh & output to bat
runshc:
    cargo run --release -- -s sh | bat -l sh

# run for zsh & output to bat
runzshc:
    cargo run --release -- -s zsh | bat -l zsh

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

bench:
    cargo bench
