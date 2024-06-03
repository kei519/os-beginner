# CMake のバージョンを設定
cmake_minimum_required(VERSION 3.22)

# プロジェクト名と使用する言語を設定
project(usb CXX C)

add_library(usb STATIC)

# コンパイルオプション
target_compile_options(usb PRIVATE
	-nostdlib
	-O2
	-Wall
	-stdlib=libc++
	-ffreestanding
	-mno-red-zone
	-fno-exceptions
	-fno-rtti
	-fno-inline-functions
	-march=x86-64
	-mabi=elf
)
target_compile_features(usb PRIVATE cxx_std_17)

# ファイルを変数 `SRC` に設定
file(GLOB_RECURSE SRC src/usb/*.cpp src/usb/*.hpp src/cxx/*.cpp src/cxx/*.hpp src/cxx/*.c)
target_sources(usb PRIVATE ${SRC})

# インクルードディレクトリの指定
target_include_directories(usb PRIVATE
	src
	src/cxx
	src/usb
	src/usb/xhci
	../../devenv/x86_64-elf/include
	../../devenv/x86_64-elf/include/c++/v1
)

# 標準ライブラリを libc++ にしたいため、clang 以外は受け付けない
# （gcc は標準ライブラリの変更ができない）
if (NOT ((CMAKE_CXX_COMPILER_ID STREQUAL "Clang") AND (CMAKE_C_COMPILER_ID STREQUAL "Clang")))
	message(FATAL_ERROR "ONLY clang is allowed")
endif()
