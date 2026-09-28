#!/bin/bash -xe
cd "$(dirname "$0")"
cargo install --path . --locked
if [[ "$(basename "$SHELL")" == fish ]]; then
    mkdir -p ~/.config/fish/completions
    gm completion > ~/.config/fish/completions/gm.fish
fi

mkdir -p dist
cp "$(command -v gm)" dist/gm
gm completion > dist/gm.fish
cat > dist/install.sh <<'EOF'
#!/bin/bash -xe
cd "$(dirname "$0")"
mkdir -p ~/.local/bin ~/.config/fish/completions
cp gm ~/.local/bin/gm
cp gm.fish ~/.config/fish/completions/gm.fish
EOF
chmod +x dist/install.sh
