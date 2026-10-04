use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=cpp/bridge.h");
    println!("cargo:rerun-if-changed=cpp/bridge.cpp");
    println!("cargo:rerun-if-changed=cpp/atomic_compat.h");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let linker_source = manifest_dir.join("../../third_party/linker");
    let compat_header = manifest_dir.join("cpp/atomic_compat.h");

    // build mcpelauncher-linker
    let linker_dst = cmake::Config::new(&linker_source)
        .build_target("linker")
        .define("BUILD_TESTING", "OFF")
        .cflag(format!(
            "-w -U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0 -include {}",
            compat_header.display()
        ))
        .cxxflag(format!(
            "-w -Wno-template-body -U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0 -include {}",
            compat_header.display()
        ))
        .build();

    let build_dir = linker_dst.join("build");
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=linker");
    println!("cargo:rustc-link-lib=dylib=z");
    println!("cargo:rustc-link-lib=dylib=pthread");
    println!("cargo:rustc-link-lib=dylib=dl");
    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=stdc++");

    // compile
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag_if_supported("-Wno-template-body")
        .warnings(false)
        .include("cpp")
        .include(linker_source.join("public_include"))
        .file("cpp/bridge.cpp")
        .compile("cork_linker_bridge");
}
