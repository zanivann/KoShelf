use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

fn main() {
    rerun_if_changed_recursive(Path::new("assets"));
    rerun_if_changed_recursive(Path::new("templates"));
    rerun_if_changed_recursive(Path::new("src"));
    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=package-lock.json");
    println!("cargo:rerun-if-env-changed=KOSHELF_SKIP_NODE_BUILD");
    println!("cargo:rerun-if-env-changed=KOSHELF_SKIP_NPM_INSTALL");
    println!("cargo:rerun-if-env-changed=KOSHELF_SKIP_FONT_DOWNLOAD");
    println!("cargo:rerun-if-env-changed=KOSHELF_FONT_CACHE_DIR");

    let skip_node_build = env_flag("KOSHELF_SKIP_NODE_BUILD");
    let skip_npm_install = env_flag("KOSHELF_SKIP_NPM_INSTALL");
    let skip_font_download = env_flag("KOSHELF_SKIP_FONT_DOWNLOAD");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    
    // Pasta persistente para guardar os assets compilados (visível no Mac e no Docker)
    let static_build_dir = Path::new("assets/static_build");
    if !static_build_dir.exists() {
        let _ = fs::create_dir_all(static_build_dir);
    }

    // --- 1. GESTÃO DO NPM (Só roda se NÃO for pular) ---
    if !skip_node_build {
        // Verifica se precisa rodar install
        let should_install = !Path::new("node_modules").exists(); 
        
        if should_install && !skip_npm_install {
            eprintln!("Installing npm dependencies...");
            let status = Command::new("npm").arg("install").status();
            if let Ok(s) = status {
                if !s.success() { panic!("npm install failed"); }
            } else {
                // Se falhar ao chamar o binário (ex: sem node), avisamos.
                println!("cargo:warning=npm not found. Skipping install.");
            }
        }

        // Compila CSS/JS e salva na pasta persistente 'assets/static_build'
        compile_assets_to_persistent_dir(static_build_dir);
    } else {
        println!("cargo:warning=Skipping Node build. Using pre-compiled assets from assets/static_build/");
    }

    // --- 2. COPIAR PARA OUT_DIR (Para o include_str! funcionar) ---
    // O Rust sempre procura no OUT_DIR, então copiamos de 'assets/static_build' para lá.
    // Isso funciona tanto no Mac (compila e copia) quanto no Docker (só copia o que o Mac gerou).
    copy_dir_all(static_build_dir, Path::new(&out_dir)).expect("Failed to copy static assets to OUT_DIR");

    // --- 3. FONTES ---
    download_fonts(&out_dir, skip_font_download);
}

// --- FUNÇÕES AUXILIARES ---

fn compile_assets_to_persistent_dir(target_dir: &Path) {
    // Tailwind Principal
    run_tailwind("Tailwind", "assets/css/input.css", &target_dir.join("compiled_style.css"));
    
    // Tailwind Calendar
    run_tailwind("Calendar", "assets/css/calendar.css", &target_dir.join("compiled_calendar.css"));
    
    // Calendar Bundle (Esbuild do CSS)
    // Nota: O arquivo css gerado pelo tailwind acima serve de entrada
    run_esbuild_css("calendar", &target_dir.join("compiled_calendar.css"), target_dir);

    // TypeScript
    run_esbuild_ts(target_dir);
}

fn run_tailwind(name: &str, input: &str, output: &Path) {
    println!("Compiling {} CSS...", name);
    let status = Command::new("npx")
        .args(["tailwindcss", "-i", input, "-o", &output.to_string_lossy(), "--minify"])
        .status();
        
    match status {
        Ok(s) if s.success() => println!("{} compiled.", name),
        _ => println!("cargo:warning=Failed to compile {} CSS (is npx installed?)", name),
    }
}

fn run_esbuild_css(name: &str, input: &Path, out_dir: &Path) {
    let outfile = out_dir.join(format!("{}.css", name));
    let _ = Command::new("npx")
        .args(["esbuild", &input.to_string_lossy(), "--bundle", "--minify", "--loader:.css=css", &format!("--outfile={}", outfile.to_string_lossy())])
        .status();
}

fn run_esbuild_ts(out_dir: &Path) {
    let ts_files = vec![
        "assets/ts/app/base.ts", "assets/ts/pages/library_list.ts", "assets/ts/pages/item_detail.ts",
        "assets/ts/pages/statistics.ts", "assets/ts/pages/recap.ts", "assets/ts/pages/calendar.ts",
        "assets/ts/app/service-worker.ts",
    ];
    
    let mut args = vec![
        "esbuild".to_string(), "--bundle".to_string(), "--format=esm".to_string(),
        "--target=es2020".to_string(), "--minify".to_string(), "--entry-names=[name]".to_string(),
        format!("--outdir={}", out_dir.to_string_lossy()),
    ];
    for f in ts_files { args.push(f.to_string()); }
    
    let _ = Command::new("npx").args(&args).status();
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    if !src.exists() {
        // Se a pasta source não existe, criamos arquivos vazios no destino para não quebrar o build
        // Isso é um fallback de emergência.
        println!("cargo:warning=Assets dir {:?} not found! Creating dummy files.", src);
        let dummies = ["compiled_style.css", "base.js", "library_list.js", "item_detail.js", 
                       "statistics.js", "calendar.css", "calendar.js", "recap.js", "service-worker.js"];
        for d in dummies {
            let _ = fs::write(dst.join(d), "/* Dummy content */");
        }
        return Ok(());
    }

    if !dst.exists() { fs::create_dir_all(dst)?; }
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn env_flag(name: &str) -> bool {
    matches!(std::env::var(name).as_deref(), Ok("1") | Ok("true") | Ok("TRUE"))
}

fn rerun_if_changed_recursive(dir: &Path) {
    if !dir.exists() { return; }
    if let Some(p) = dir.to_str() { println!("cargo:rerun-if-changed={}", p); }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() { rerun_if_changed_recursive(&entry.path()); }
            else if let Some(p) = entry.path().to_str() { println!("cargo:rerun-if-changed={}", p); }
        }
    }
}

fn download_fonts(out_dir: &str, skip_download: bool) {
    let fonts = [
        ("Gelasio-Regular.ttf", "https://fonts.gstatic.com/s/gelasio/v14/cIfiMaFfvUQxTTqS3iKJkLGbI41wQL8Ilycs.ttf"),
        ("Gelasio-Italic.ttf", "https://fonts.gstatic.com/s/gelasio/v14/cIfsMaFfvUQxTTqS9Cu7b2nySBfeR6rA1M9v8zQ.ttf"),
    ];
    let cache_dir = std::env::var("KOSHELF_FONT_CACHE_DIR").map(std::path::PathBuf::from).unwrap_or_else(|_| Path::new("target").join(".font-cache"));
    let _ = fs::create_dir_all(&cache_dir);

    for (filename, url) in fonts {
        let cache_path = cache_dir.join(filename);
        let dest_path = Path::new(out_dir).join(filename);

        if cache_path.exists() {
            if let Ok(bytes) = fs::read(&cache_path) {
                let _ = fs::write(&dest_path, bytes);
                continue;
            }
        }
        if skip_download { continue; }

        if let Ok(response) = ureq::get(url).call() {
            if let Ok(bytes) = response.into_body().read_to_vec() {
                let _ = fs::write(&cache_path, &bytes);
                let _ = fs::write(&dest_path, bytes);
                println!("cargo:warning=Font {} downloaded.", filename);
            }
        }
    }
}