#!/bin/sh
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --release --target=wasm32-unknown-unknown && \
wasm-bindgen target/wasm32-unknown-unknown/release/unimusic-sync.wasm \
    --target=web\
    --out-dir=out