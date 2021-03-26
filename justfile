#!/usr/bin/env just --justfile
bin_name := 'shellenv'
dev := '1'

alias r := run
alias b := build
alias i := install
alias h := help
alias file-run := run

# build release binary
build:
    cargo build

# install release binary to $HOME/.cargo/bin
install:
    cargo install -f --path .

# rebuild docs
doc:
    cargo doc

# rebuild docs and start simple static server
docs +PORT='40000':
    cargo doc && http target/doc -p {{PORT}}

# start server for docs and update upon changes
docslive:
    light-server -c .lightrc

# rebuild docs and start simple static server that watches for changes (in parallel)
docw +PORT='40000':
    parallel --lb ::: "cargo watch -x 'doc --color=always'" "http target/doc -p {{PORT}}"

# build release binary and run
run +args='':
    cargo run -- {{args}}

help:
    ./target/debug/{{bin_name}} -h

# run binary
rb +args='':
    ./target/debug/{{bin_name}} {{args}}

runc:
    #!/usr/bin/env bash
    shell=$(basename $SHELL)
    cargo run -- -s $shell | bat -l $shell

# run with -v
runv:
    cargo run -- -v

# run with -vv
runvv:
    cargo run -- -vv

# run for (ba)sh
runsh +args='':
    cargo run -- -s sh {{args}}

# run for zsh
runzsh +args='':
    cargo run -- -s zsh {{args}}

# run for powershell
runps +args='':
    cargo run -- -s ps {{args}}

# run for fish
runfish +args='':
    cargo run -- -s fish {{args}}

# run for (ba)sh & output to bat
runshc:
    cargo run -- -s sh | bat -l sh

# run for zsh & output to bat
runzshc:
    cargo run -- -s zsh | bat -l zsh

# run for powershell & output to bat
runpsc:
    cargo run -- -s ps | bat -l powershell

# run for fish & output to bat
runfishc:
    cargo run -- -s fish | bat -l fish

test:
    cargo test

fix:
    cargo fix

bench:
    cargo bench
