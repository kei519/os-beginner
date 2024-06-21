TARGET = rpn_cpp

CPPFLAGS += -I.
CFLAGS += -O2 -Wall -g --target=x86_64-elf -ffreestanding
CXXFLAGS += -O2 -Wall -g --target=x86_64-elf -ffreestanding \
			-fno-exceptions -fno-rtti -std=c++17

LDFLAGS += --entry main -z norelro --image-base=0 --static

.PHONY: all
all: $(TARGET)

rpn_cpp: rpn.o Makefile
	ld.lld $(LDFLAGS) -o rpn_cpp rpn.o

rpn.o: rpn.cpp
	clang++ $(CPPFLAGS) $(CXXFLAGS) -c $< -o $@
