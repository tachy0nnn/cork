use std::path::Path;
use std::process::Command;

fn main() {
    // commit hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string());

    let pkg_version = env!("CARGO_PKG_VERSION");
    let full_version = format!("{pkg_version}-{git_hash}");
    println!("cargo:rustc-env=GIT_HASH={git_hash}");
    println!("cargo:rustc-env=BUILD_VERSION={full_version}");

    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        && output.status.success()
    {
        let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let head_path = Path::new(&git_dir).join("HEAD");
        if head_path.exists() {
            println!("cargo:rerun-if-changed={}", head_path.display());
            if let Ok(head_contents) = std::fs::read_to_string(&head_path)
                && let Some(ref_path) = head_contents.strip_prefix("ref: ")
            {
                let ref_file = Path::new(&git_dir).join(ref_path.trim());
                if ref_file.exists() {
                    println!("cargo:rerun-if-changed={}", ref_file.display());
                }
            }
        }
    }
}
