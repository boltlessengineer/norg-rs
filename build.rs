use std::process::Command;

fn main() {
    let output = Command::new("janet")
        .arg("-c")
        .arg("janet-src/stdlib.janet")
        .output()
        .expect("Failed to execute janet command");
    if !output.status.success() {
        panic!("janet command failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=janet-src");
}
