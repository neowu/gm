#!/bin/bash -xe
cd "$(dirname "$0")"
cargo install --path . --locked
if [[ "$(basename "$SHELL")" == fish ]]; then
    mkdir -p ~/.config/fish/completions
    gm completion > ~/.config/fish/completions/gm.fish
fi
