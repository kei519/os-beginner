TARGET = stack

.PHONY: all
all: $(TARGET)

$(TARGET): src/*.rs Makefile .cargo/config.toml Cargo.toml
	cargo build --release
	cp target/x86_64-unknown-none/release/$(TARGET) ./
