use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Tell Cargo to re-run this script if the compute shader changes
    println!("cargo:rerun-if-changed=src/shaders/gradient.slang");

    // Grab Cargo's standard build output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = PathBuf::from(out_dir).join("gradient.spv");

    // Invoke the slangc CLI compiler
    let output = Command::new("slangc")
        .args(&[
            "src/shaders/gradient.slang",
            "-target", "spirv", // Target Vulkan SPIR-V
            "-o", dest_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute slangc. Is it installed and in your PATH?");

    // If the shader has syntax errors, crash the build and print the compiler logs
    if !output.status.success() {
        panic!(
            "Shader compilation failed!\nCompiler Output:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}