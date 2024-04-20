use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to link the static libraries
    println!("cargo:rustc-link-lib=static=hfuzz");
    println!("cargo:rustc-link-lib=static=hfcommon");

    // Specify the path to the static libraries
    let lib_dir = PathBuf::from("/tmp/honggfuzz/libhfuzz");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    let lib_dir = PathBuf::from("/tmp/honggfuzz/libhfcommon/");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
}
