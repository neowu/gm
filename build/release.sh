#!/bin/bash -xe
cargo build --release

sudo cp ./target/release/gm /usr/local/bin
# for fish only
gm completion > ~/.config/fish/completions/gm.fish
