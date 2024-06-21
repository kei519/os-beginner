TARGET = rpn

.PHONY: all
all: $(TARGET)

rpn: src/*.rs Makefile .cargo/config.toml Cargo.toml
	cargo build --release
	cp target/x86_64-unknown-none/release/rpn ./
