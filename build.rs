use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let shader_dir = "src/shaders";

    // Powiedz Cargo, żeby uruchomił skrypt ponownie, jeśli zawartość folderu się zmieni
    // (np. gdy dodasz lub usuniesz plik shadera)
    println!("cargo:rerun-if-changed={}", shader_dir);

    // Pobierz folder wyjściowy Cargo
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(out_dir);

    // Przeczytaj zawartość folderu z shaderami
    let entries = fs::read_dir(shader_dir).expect("Failed to read shaders directory");

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        // Przetwarzaj tylko pliki z rozszerzeniem .slang
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("slang") {
            // Powiedz Cargo, żeby przebudował projekt, jeśli ten konkretny plik się zmieni
            println!("cargo:rerun-if-changed={}", path.display());

            // Pobierz nazwę pliku bez rozszerzenia (np. "gradient" z "gradient.slang")
            let file_stem = path.file_stem().unwrap().to_str().unwrap();

            // Przygotuj ścieżkę wyjściową (np. OUT_DIR/gradient.spv)
            let dest_path = out_path.join(format!("{}.spv", file_stem));

            // Uruchom kompilator slangc dla tego pliku
            let output = Command::new("slangc")
                .args(&[
                    path.to_str().unwrap(),
                    "-target", "spirv", // Target Vulkan SPIR-V
                    "-o", dest_path.to_str().unwrap(),
                ])
                .output()
                .expect("Failed to execute slangc. Is it installed and in your PATH?");

            // Jeśli wystąpią błędy składniowe w shaderze, zatrzymaj budowanie i wypisz logi
            if !output.status.success() {
                panic!(
                    "Shader compilation failed for file: {:?}!\nCompiler Output:\n{}",
                    path,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}