//! Internationalization using fluent-bundle.

use include_dir::{include_dir, Dir};
use std::sync::RwLock;
use once_cell::sync::Lazy;
use serde::Serialize; // Necessário para enviar JSON estruturado

pub mod translations;

// Mantém o que já existia
pub use translations::{Translations, list_supported_languages};

// Incorpora a pasta 'locales' dentro do binário.
static LOCALES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/locales");

/// Estrutura para enviar ao Frontend
#[derive(Debug, Serialize, Clone)]
pub struct LanguageInfo {
    pub code: String, // ex: "pt", "en"
    pub name: String, // ex: "Português (Brasil)", "English"
}

/// Retorna a lista de idiomas com metadados extraídos dos arquivos .ftl
pub fn get_available_locales() -> Vec<LanguageInfo> {
    let mut locales = Vec::new();

    // Itera sobre os arquivos embutidos
    for file in LOCALES_DIR.files() {
        let path = file.path();

        // Filtra apenas arquivos com extensão .ftl
        if path.extension().map_or(false, |ext| ext == "ftl") {
            // Pega o código base do nome do arquivo (ex: "pt.ftl" -> "pt")
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let code = stem.to_string();
                
                // Tenta ler o nome amigável de dentro do arquivo
                let content = file.contents_utf8().unwrap_or("");
                let mut name = code.to_uppercase(); // Fallback padrão (ex: PT)

                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("-lang-name") {
                        if let Some((_, value)) = line.split_once('=') {
                            name = value.trim().to_string();
                            break; // Encontrou o nome, pode parar de ler este arquivo
                        }
                    }
                }

                locales.push(LanguageInfo { code, name });
            }
        }
    }

    // Ordena alfabeticamente pelo NOME para ficar bonito no menu
    locales.sort_by(|a, b| a.name.cmp(&b.name));
    
    locales
}

// Variável global para controlar o idioma atual do sistema
static CURRENT_LOCALE: Lazy<RwLock<String>> = Lazy::new(|| RwLock::new("en".to_string()));

pub fn set_global_locale(lang: String) {
    match CURRENT_LOCALE.write() {
        Ok(mut w) => *w = lang,
        Err(e) => {
            let mut w = e.into_inner();
            *w = lang;
        }
    }
}

pub fn get_global_locale() -> String {
    match CURRENT_LOCALE.read() {
        Ok(r) => r.clone(),
        Err(e) => e.into_inner().clone(),
    }
}