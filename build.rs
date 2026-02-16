use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    rerun_if_changed_recursive(Path::new("assets"));
    rerun_if_changed_recursive(Path::new("templates"));
    rerun_if_changed_recursive(Path::new("src"));
    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=package-lock.json");

    // Variáveis de ambiente para controle do CI/Cross
    println!("cargo:rerun-if-env-changed=KOSHELF_SKIP_NODE_BUILD");
    println!("cargo:rerun-if-env-changed=KOSHELF_SKIP_NPM_INSTALL");
    println!("cargo:rerun-if-env-changed=KOSHELF_SKIP_FONT_DOWNLOAD");
    println!("cargo:rerun-if-env-changed=KOSHELF_FONT_CACHE_DIR");

    let out_dir = env::var("OUT_DIR").unwrap();
    let skip_node_build = env_flag("KOSHELF_SKIP_NODE_BUILD");
    let skip_npm_install = env_flag("KOSHELF_SKIP_NPM_INSTALL");
    let skip_font_download = env_flag("KOSHELF_SKIP_FONT_DOWNLOAD");

    // =========================================================================
    // LÓGICA DE INJEÇÃO (PARA CROSS/CI)
    // =========================================================================
    if skip_node_build {
        eprintln!("KOSHELF_SKIP_NODE_BUILD is set. Skipping npm/build steps.");
        eprintln!("Looking for prebuilt assets in 'prebuilt/' directory...");

        let prebuilt_dir = Path::new("prebuilt");
        let out_path = Path::new(&out_dir);

        let expected_files = [
            "compiled_style.css",
            "compiled_calendar.css",
            "calendar.css",
            "base.js",
            "library_list.js",
            "item_detail.js",
            "statistics.js",
            "recap.js",
            "calendar.js",
            "service-worker.js",
        ];

        for file_name in expected_files {
            let src = prebuilt_dir.join(file_name);
            let dest = out_path.join(file_name);

            if src.exists() {
                fs::copy(&src, &dest)
                    .unwrap_or_else(|e| panic!("Failed to copy prebuilt asset {}: {}", file_name, e));
                eprintln!("Copied prebuilt asset: {}", file_name);
            } else {
                eprintln!(
                    "WARNING: Prebuilt asset '{}' not found. Creating dummy file.",
                    file_name
                );
                fs::write(&dest, "").expect("Failed to create dummy file");
            }
        }

        // --- CORREÇÃO AQUI ---
        // Usamos .is_err() em vez de tentar casar o padrão std::panic::Result
        if std::panic::catch_unwind(|| {
            download_fonts(&out_dir, skip_font_download);
        })
        .is_err()
        {
            eprintln!(
                "WARNING: Failed to download fonts inside Cross container. Continuing without them."
            );
        }

        return; // ENCERRA AQUI
    }

    // =========================================================================
    // LÓGICA PADRÃO (DESENVOLVIMENTO LOCAL)
    // =========================================================================

    if !Path::new("package.json").exists() {
        panic!("package.json not found. Please ensure Tailwind CSS dependencies are configured.");
    }

    let should_install = !Path::new("node_modules").exists()
        || (Path::new("package-lock.json").exists()
            && Path::new("node_modules")
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                < Path::new("package-lock.json")
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH));

    if should_install && !skip_npm_install {
        eprintln!("Installing npm dependencies...");
        let mut cmd = Command::new("npm");
        if Path::new("package-lock.json").exists() {
            cmd.arg("ci");
        } else {
            cmd.arg("install");
        }
        let install_output = cmd
            .output()
            .expect("Failed to run npm install. Make sure Node.js and npm are installed.");

        if !install_output.status.success() {
            panic!(
                "npm install failed: {}",
                String::from_utf8_lossy(&install_output.stderr)
            );
        }
        eprintln!("npm install completed successfully");
    } else if should_install && skip_npm_install {
        panic!("node_modules missing/outdated but npm install is disabled.");
    }

    compile_tailwind_css(
        &out_dir,
        "Tailwind",
        Path::new("assets/css/input.css"),
        "compiled_style.css",
    );

    let compiled_calendar = compile_tailwind_css(
        &out_dir,
        "calendar Tailwind",
        Path::new("assets/css/calendar.css"),
        "compiled_calendar.css",
    );
    bundle_css_with_esbuild("calendar", &compiled_calendar, &out_dir);

    compile_typescript(&out_dir);

    download_fonts(&out_dir, skip_font_download);
}

// -----------------------------------------------------------------------------
// FUNÇÕES AUXILIARES
// -----------------------------------------------------------------------------

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES") | Ok("on") | Ok("ON")
    )
}

fn rerun_if_changed_recursive(dir: &Path) {
    if !dir.exists() {
        return;
    }
    if let Some(p) = dir.to_str() {
        println!("cargo:rerun-if-changed={}", p);
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rerun_if_changed_recursive(&path);
        } else if path.is_file() {
            if let Some(p) = path.to_str() {
                println!("cargo:rerun-if-changed={}", p);
            }
        }
    }
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> io::Result<bool> {
    match fs::read(path) {
        Ok(existing) if existing == bytes => Ok(false),
        _ => {
            fs::write(path, bytes)?;
            Ok(true)
        }
    }
}

fn compile_tailwind_css(
    out_dir: &str,
    display_name: &str,
    input: &Path,
    out_filename: &str,
) -> PathBuf {
    if !input.exists() {
        panic!("{} not found", input.display());
    }

    eprintln!("Compiling {} CSS...", display_name);
    let tmp_path = Path::new(out_dir).join(format!("{}.tmp", out_filename));
    let dest_path = Path::new(out_dir).join(out_filename);

    let output = Command::new("npx")
        .args([
            "tailwindcss",
            "-i",
            &input.to_string_lossy(),
            "-o",
            &tmp_path.to_string_lossy(),
            "--minify",
        ])
        .output()
        .expect("Failed to run Tailwind CSS.");

    if !output.status.success() {
        panic!(
            "{} CSS compilation failed:\nstderr: {}",
            display_name,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let css_bytes = fs::read(&tmp_path).expect("Failed to read generated CSS");
    write_if_changed(&dest_path, &css_bytes).expect("Failed to write output");
    let _ = fs::remove_file(&tmp_path);
    dest_path
}

fn bundle_css_with_esbuild(name: &str, input_path: &Path, out_dir: &str) {
    if !input_path.exists() {
        panic!("{} not found", input_path.display());
    }

    eprintln!("Bundling {} CSS...", name);
    let output_name = format!("{}.css", name);
    let outfile = Path::new(out_dir).join(&output_name);
    let tmpfile = Path::new(out_dir).join(format!("{}.esbuild.tmp", name));

    let output = Command::new("npx")
        .args([
            "esbuild",
            &input_path.to_string_lossy(),
            "--bundle",
            "--minify",
            "--loader:.css=css",
            &format!("--outfile={}", tmpfile.to_string_lossy()),
        ])
        .output()
        .expect("Failed to run esbuild.");

    if !output.status.success() {
        panic!(
            "{} CSS bundling failed:\nstderr: {}",
            name,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let css_bytes = fs::read(&tmpfile).expect("Failed to read bundled CSS");
    write_if_changed(&outfile, &css_bytes).expect("Failed to write bundled CSS");
    let _ = fs::remove_file(&tmpfile);
}

fn compile_typescript(out_dir: &str) {
    let ts_dir = Path::new("assets/ts");
    if !ts_dir.exists() {
        return;
    }

    let ts_files = vec![
        "assets/ts/app/base.ts",
        "assets/ts/pages/library_list.ts",
        "assets/ts/pages/item_detail.ts",
        "assets/ts/pages/statistics.ts",
        "assets/ts/pages/recap.ts",
        "assets/ts/pages/calendar.ts",
        "assets/ts/app/service-worker.ts",
    ];

    let mut args = vec![
        "esbuild".to_string(),
        "--bundle".to_string(),
        "--format=esm".to_string(),
        "--target=es2020".to_string(),
        "--minify".to_string(),
        "--entry-names=[name]".to_string(),
        format!("--outdir={}", out_dir),
    ];
    args.extend(ts_files.iter().map(|s| s.to_string()));

    let output = Command::new("npx")
        .args(&args)
        .output()
        .expect("Failed to run esbuild for TS.");

    if !output.status.success() {
        panic!(
            "TS compilation failed:\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn download_fonts(out_dir: &str, skip_download: bool) {
    let fonts = [
        (
            "Gelasio-Regular.ttf",
            "https://fonts.gstatic.com/s/gelasio/v14/cIfiMaFfvUQxTTqS3iKJkLGbI41wQL8Ilycs.ttf",
        ),
        (
            "Gelasio-Italic.ttf",
            "https://fonts.gstatic.com/s/gelasio/v14/cIfsMaFfvUQxTTqS9Cu7b2nySBfeR6rA1M9v8zQ.ttf",
        ),
    ];

    let cache_dir = env::var("KOSHELF_FONT_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new("target").join(".font-cache"));

    fs::create_dir_all(&cache_dir).unwrap_or_default();

    for (filename, url) in fonts {
        let cache_path = cache_dir.join(filename);
        let dest_path = Path::new(out_dir).join(filename);

        if cache_path.exists() {
            if let Ok(bytes) = fs::read(&cache_path) {
                let _ = write_if_changed(&dest_path, &bytes);
                continue;
            }
        }

        if skip_download {
            continue;
        }

        match ureq::get(url).call() {
            Ok(response) => {
                if let Ok(bytes) = response.into_body().read_to_vec() {
                    let _ = fs::write(&cache_path, &bytes);
                    let _ = write_if_changed(&dest_path, &bytes);
                    eprintln!("Downloaded font: {}", filename);
                }
            }
            Err(e) => eprintln!("Failed to download font {}: {}", filename, e),
        }
    }
}