# CMake のバージョンを設定
cmake_minimum_required(VERSION 3.22)

# クロスコンパイル時にリンクしないように設定
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)

# プロジェクト名と使用する言語を設定
project(usb CXX C)

# target triple の設定
set(CMAKE_SYSTEM_NAME Generic-ELF)
set(CMAKE_SYSTEM_PROCESSOR x86_64)
set(CMAKE_C_COMPILER_TARGET x86_64-unknown-none)
set(CMAKE_CXX_COMPILER_TARGET x86_64-unknown-none)

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
