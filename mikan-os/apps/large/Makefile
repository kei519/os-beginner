TARGET = large

.PHONY: all
all: $(TARGET)

large: src/*.rs Makefile .cargo/config.toml Cargo.toml
	cargo build --release
	cp target/x86_64-unknown-none/release/large ./
