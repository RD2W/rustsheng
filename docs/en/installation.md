# Installation & building

## Requirements

- **Rust ≥ 1.97** (the workspace uses `edition = "2024"`). Install via
  [rustup](https://rustup.rs/).
- A serial port / USB-to-UART programming cable (e.g. a CH340-based UV-K5 cable).
- **Linux only:** `libudev` development files are needed to build the
  `serialport` dependency and to enumerate ports:
  ```bash
  sudo apt-get install -y libudev-dev pkg-config
  ```

## Build

```bash
# Debug build of the whole workspace
cargo build --workspace

# Optimized release build (recommended)
cargo build --release --workspace
# or:
make build
```

The CLI binary is produced at `target/release/rustsheng`.

## Cargo features (core library)

The `rustsheng-core` crate has two features, both **on by default**:

| Feature | Enables | Extra dependencies |
|---------|---------|--------------------|
| `serial` | the real `SerialTransport` (serial-port I/O) | `serialport` |
| `flash-v5` | AES-CBC-128 firmware flashing (bootloader V5) | `aes`, `cbc` |

Build the core without them (e.g. to reuse the pure protocol layer elsewhere):

```bash
cargo build -p rustsheng-core --no-default-features
```

Without `flash-v5`, requesting a V5 flash returns `FlashError::V5Unavailable`.

## Running the tests

```bash
cargo test --workspace
# or:
make test
```

The suite is hardware-free: the protocol and domain layers are pure, and the
client is exercised through an in-memory `MockTransport`. Integration tests read
reference `.raw` fixtures committed under
`crates/rustsheng-core/tests/fw/`.

## Cross-compilation

The `Makefile` wraps `cargo` for cross-builds:

```bash
make install-targets                    # rustup target add ...
make build-all                          # all targets for the host platform
make build-platform PLATFORM=windows    # a single platform
```

CI (`.github/workflows/release.yml`) builds Linux (x86_64, aarch64 via `cross`),
Windows (x86_64 via mingw-w64) and macOS (x86_64, aarch64) on tag pushes.

## Packaging

`make package` builds the release binary and archives it with the version from
`Cargo.toml`:

```bash
make version    # prints e.g. 0.1.0
make package    # builds/rustsheng_v0.1.0_<platform>.tar.gz (or .zip on Windows)
```
