#!/bin/bash -xe
cargo install --path .
# for fish only
gm completion > ~/.config/fish/completions/gm.fish
