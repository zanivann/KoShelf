use std::{env, fs, path::Path};

fn main() {
    // Rebuild se algo mudar no diretório de assets finais
    println!("cargo:rerun-if-changed=assets_dist");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);

    let assets_dir = Path::new("assets_dist");

    if !assets_dir.exists() {
        panic!(
            "assets_dist directory not found.\n\
             You must run the frontend build before compiling:\n\
             ./scripts/build-frontend.sh"
        );
    }

    for entry in fs::read_dir(assets_dir).expect("Failed to read assets_dist") {
        let entry = entry.expect("Invalid dir entry");
        let src = entry.path();

        if src.is_file() {
            let file_name = src
                .file_name()
                .expect("Invalid filename");

            let dest = out_path.join(file_name);

            fs::copy(&src, &dest).unwrap_or_else(|e| {
                panic!(
                    "Failed to copy asset {:?} → {:?}: {}",
                    src, dest, e
                )
            });
        }
    }
}
