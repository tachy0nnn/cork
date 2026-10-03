use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=cpp/bridge.h");
    println!("cargo:rerun-if-changed=cpp/bridge.cpp");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let jnivm_source = manifest_dir.join("../../third_party/libjnivm");

    // build via cmake
    let jnivm_dst = cmake::Config::new(&jnivm_source)
        .build_target("jnivm")
        .define("JNIVM_ENABLE_TESTS", "OFF")
        .define("JNIVM_BUILD_EXAMPLES", "OFF")
        .define("JNIVM_ENABLE_DEBUG", "OFF")
        .cflag("-include string -w")
        .cxxflag("-include string -w -Wno-template-body")
        .build();

    let build_dir = jnivm_dst.join("build");
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!(
        "cargo:rustc-link-search=native={}",
        build_dir.join("src").display()
    );
    println!("cargo:rustc-link-lib=static=jnivm");
    println!("cargo:rustc-link-lib=dylib=stdc++");

    // compile
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .define("EnableJNIVMGC", None)
        .flag("-include")
        .flag("string")
        .flag_if_supported("-Wno-template-body")
        .warnings(false)
        .include("cpp")
        .include(jnivm_source.join("include"))
        .file("cpp/bridge.cpp")
        .compile("cork_jnivm_bridge");
}
