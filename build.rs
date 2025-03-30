use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("embed.jimage");
    let janet_script = format!(r#"
        (do
          (def env (make-env))
          (def entry-env (dofile "janet-src/embed.janet" :env env))
          (def- main ((entry-env 'main) :value))
          (def mdict (invert (env-lookup root-env)))
          (def image (marshal main mdict))

          (def out (file/open ```{}``` :wn))
          (file/write out image)
          (file/close out))
    "#, dest_path.to_str().unwrap());
    let output = Command::new("janet")
        .arg("-e")
        .arg(janet_script)
        .env("JANET_PATH", "jpm_tree/lib")
        .output()
        .expect("Failed to execute janet command");
    if !output.status.success() {
        panic!("janet command failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    println!("cargo::rerun-if-changed=build.rs");
}
