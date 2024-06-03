use cmake::Config;

fn main() {
    if cfg!(feature = "not-check") {
        let dst = Config::new("./")
            .define("CMAKE_C_COMPILER", "clang")
            .define("CMAKE_CXX_COMPILER", "clang++")
            .build_target("usb")
            .build();

        println!(
            "cargo:rustc-link-search=native={}",
            dst.join("build").display()
        );
        println!("cargo:rustc-link-lib=static=usb");

        // C++ のファイルの内容も .rlib にキャッシュ？されるらしく、
        // そのコンパイル時にライブラリが必要になるため書いている
        // ただ、あんまり良くわかっていない
        println!("cargo:rustc-link-search=native=../../devenv/x86_64-elf/lib");
        println!("cargo:rustc-link-lib=static=c");
        println!("cargo:rustc-link-lib=static=c++abi");
    }
}
