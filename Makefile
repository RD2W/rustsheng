# Makefile для сборки под разные платформы (Windows, Linux, Mac)

# Определение текущей платформы
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

# Определение целей для разных архитектур
ifeq ($(PLATFORM),windows)
    TARGETS = x86_64-pc-windows-gnu i686-pc-windows-gnu
else ifeq ($(PLATFORM),linux)
    TARGETS = x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
else ifeq ($(PLATFORM),mac)
    TARGETS = x86_64-apple-darwin aarch64-apple-darwin
endif

# Имя бинарника (предполагаем, что определено в Cargo.toml)
BINARY_NAME := $(shell grep '^name' Cargo.toml | head -n1 | cut -d'=' -f2 | tr -d ' ' | tr -d '"')

.PHONY: all build clean install-deps test run help

# Цель по умолчанию
all: build

# Сборка для текущей платформы
build:
	cargo build --release

# Сборка для всех поддерживаемых платформ
build-all: install-targets
	@for target in $(TARGETS); do \
		echo "Сборка для $$target"; \
		cargo build --release --target $$target; \
	done

# Сборка для конкретной платформы (например, make build-platform PLATFORM=windows)
build-platform:
	@if [ -z "$(PLATFORM)" ]; then \
		echo "Пожалуйста, укажите платформу: make build-platform PLATFORM=windows"; \
		exit 1; \
	fi
	@if [ "$(PLATFORM)" = "windows" ]; then \
		cargo build --release --target x86_64-pc-windows-gnu; \
	elif [ "$(PLATFORM)" = "linux" ]; then \
		cargo build --release --target x86_64-unknown-linux-gnu; \
	elif [ "$(PLATFORM)" = "mac" ]; then \
		cargo build --release --target x86_64-apple-darwin; \
	fi

# Установка целей для кросс-компиляции
install-targets:
	@for target in $(TARGETS); do \
		echo "Установка цели $$target"; \
		rustup target add $$target; \
	done

# Очистка скомпилированных файлов
clean:
	cargo clean

# Установка зависимостей (rustup и т.д.)
install-deps:
	@if ! command -v rustc >/dev/null 2>&1; then \
		echo "Rust не установлен. Установите его с помощью rustup:"; \
		echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; \
		exit 1; \
	fi
	cargo check

# Запуск тестов
test:
	cargo test

# Запуск приложения
run:
	cargo run

# Показать справку по целям
help:
	@echo "Доступные цели:"
	@echo "  all              - Сборка для текущей платформы (по умолчанию)"
	@echo "  build            - Сборка для текущей платформы"
	@echo "  build-all        - Сборка для всех поддерживаемых платформ"
	@echo "  build-platform   - Сборка для конкретной платформы (используйте PLATFORM=...)"
	@echo "  install-targets  - Установка целей для кросс-компиляции"
	@echo "  clean            - Очистка скомпилированных файлов"
	@echo "  install-deps     - Установка зависимостей"
	@echo "  test             - Запуск тестов"
	@echo "  run              - Запуск приложения"
	@echo "  help             - Показать это сообщение"
	@echo ""
	@echo "Примеры:"
	@echo "  make build-all"
	@echo "  make build-platform PLATFORM=windows"
	@echo "  make test"