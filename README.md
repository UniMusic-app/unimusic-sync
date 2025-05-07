# Prerequisites (for WASM)

## All

```
cargo install wasm-bindgen-cli
```

## macOS

On macOS you might need to use clang version shipped with `llvm` package

```bash
brew install llvm
```

And then add it to your path:

- sh/zsh/bash:
  ```bash
  export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
  ```
- fish:
  ```fish
  fish_add_path "/opt/homebrew/opt/llvm/bin"
  ```
