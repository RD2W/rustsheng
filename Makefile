# Makefile for building on multiple platforms (Windows, Linux, Mac)

# Detect the current platform
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
    PLATFORM = linux
    EXT =
endif
ifeq ($(UNAME_S),Darwin)
    PLATFORM = mac
    EXT =
endif
ifeq ($(findstring MINGW,$(UNAME_S)),MINGW)
    PLATFORM = windows
    EXT = .exe
endif

# Cross-compilation targets per platform
ifeq ($(PLATFORM),windows)
    TARGETS = x86_64-pc-windows-gnu i686-pc-windows-gnu
else ifeq ($(PLATFORM),linux)
    TARGETS = x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
else ifeq ($(PLATFORM),mac)
    TARGETS = x86_64-apple-darwin aarch64-apple-darwin
endif

# CLI binary name and version (version is read from the workspace Cargo.toml)
BINARY_NAME = rustsheng
VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
OUTPUT_DIR = builds
ARCHIVE_PREFIX = $(BINARY_NAME)_v$(VERSION)

.PHONY: all build build-all build-platform install-targets clean install-deps test run help version package

# Default target
all: build

# Build the whole workspace for the current platform
build:
	cargo build --release --workspace

# Build the workspace for every supported target
build-all: install-targets
	@for target in $(TARGETS); do \
		echo "Building for $$target"; \
		cargo build --release --workspace --target $$target; \
	done

# Build for a specific platform (e.g. make build-platform PLATFORM=windows)
build-platform:
	@if [ -z "$(PLATFORM)" ]; then \
		echo "Please specify a platform: make build-platform PLATFORM=windows"; \
		exit 1; \
	fi
	@if [ "$(PLATFORM)" = "windows" ]; then \
		cargo build --release --workspace --target x86_64-pc-windows-gnu; \
	elif [ "$(PLATFORM)" = "linux" ]; then \
		cargo build --release --workspace --target x86_64-unknown-linux-gnu; \
	elif [ "$(PLATFORM)" = "mac" ]; then \
		cargo build --release --workspace --target x86_64-apple-darwin; \
	fi

# Install cross-compilation targets
install-targets:
	@for target in $(TARGETS); do \
		echo "Installing target $$target"; \
		rustup target add $$target; \
	done

# Remove build artifacts
clean:
	cargo clean

# Check the toolchain and dependencies
install-deps:
	@if ! command -v rustc >/dev/null 2>&1; then \
		echo "Rust is not installed. Install it with rustup:"; \
		echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; \
		exit 1; \
	fi
	cargo check --workspace

# Run the test suite
test:
	cargo test --workspace

# Run the CLI binary
run:
	cargo run -p rustsheng --

# Print the version read from Cargo.toml
version:
	@echo $(VERSION)

# Build the current-platform release binary and archive it with the version
# in the file name (e.g. builds/rustsheng_v0.1.0_linux.tar.gz).
package: build
	@mkdir -p $(OUTPUT_DIR)
	@cp target/release/$(BINARY_NAME)$(EXT) $(OUTPUT_DIR)/$(BINARY_NAME)$(EXT)
	@if [ "$(PLATFORM)" = "windows" ]; then \
		cd $(OUTPUT_DIR) && zip -q $(ARCHIVE_PREFIX)_$(PLATFORM).zip $(BINARY_NAME)$(EXT); \
	else \
		tar -czf $(OUTPUT_DIR)/$(ARCHIVE_PREFIX)_$(PLATFORM).tar.gz -C $(OUTPUT_DIR) $(BINARY_NAME)$(EXT); \
	fi
	@echo "Packaged $(OUTPUT_DIR)/$(ARCHIVE_PREFIX)_$(PLATFORM)"

# Show available targets
help:
	@echo "Available targets:"
	@echo "  all              - Build for the current platform (default)"
	@echo "  build            - Build the workspace for the current platform"
	@echo "  build-all        - Build for all supported targets"
	@echo "  build-platform   - Build for a specific platform (use PLATFORM=...)"
	@echo "  install-targets  - Install cross-compilation targets"
	@echo "  clean            - Remove build artifacts"
	@echo "  install-deps     - Check the toolchain and dependencies"
	@echo "  test             - Run the test suite"
	@echo "  run              - Run the CLI binary"
	@echo "  version          - Print the version from Cargo.toml"
	@echo "  package          - Build and archive the current-platform binary"
	@echo "  help             - Show this message"
	@echo ""
	@echo "Examples:"
	@echo "  make build-all"
	@echo "  make build-platform PLATFORM=windows"
	@echo "  make test"
