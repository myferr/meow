###  Usage

```bash
make macos-x64        # Builds for Intel macOS
make macos-arm        # Builds for Apple Silicon
make linux            # Builds for Linux via cross
make windows          # Builds for Windows via cross
make all              # Builds all
```

---

### Prerequisites

* Install targets:

  ```bash
  rustup target add x86_64-apple-darwin aarch64-apple-darwin x86_64-unknown-linux-gnu x86_64-pc-windows-msvc
  ```

* Install [`cross`](https://github.com/cross-rs/cross):

  ```bash
  cargo install cross
  ```
