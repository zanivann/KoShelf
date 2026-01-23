use anyhow::{Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::koreader::{LuaParser, calculate_partial_md5};
use crate::models::{BookInfo, KoReaderMetadata, LibraryItem, LibraryItemFormat};
use crate::parsers::{ComicParser, EpubParser, Fb2Parser, MobiParser};
use crate::utils::generate_book_id;
use crate::config::SiteConfig;
use crate::time_config::TimeConfig;

/// Configuration for where to find KOReader metadata
#[derive(Clone, Debug, Default)]
pub enum MetadataLocation {
    /// Default: metadata stored in .sdr folder next to each book
    #[default]
    InBookFolder,
    /// Metadata stored in docsettings folder with full path structure
    DocSettings(PathBuf),
    /// Metadata stored in hashdocsettings folder organized by partial MD5 hash
    HashDocSettings(PathBuf),
}

// Funções de indexação para suportar diferentes localizações de metadados
fn build_docsettings_index(docsettings_path: &PathBuf) -> Result<HashMap<String, PathBuf>> {
    let mut index: HashMap<String, PathBuf> = HashMap::new();
    let mut duplicates: Vec<String> = Vec::new();
    info!("Scanning docsettings folder: {:?}", docsettings_path);

    for entry in walkdir::WalkDir::new(docsettings_path) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => { warn!("Failed to read entry: {}", e); continue; }
        };
        let path = entry.path();
        if path.is_dir()
            && let Some(dir_name) = path.file_name().and_then(|s| s.to_str())
            && let Some(book_stem) = dir_name.strip_suffix(".sdr")
        {
            let epub_metadata_path = path.join("metadata.epub.lua");
            let fb2_metadata_path = path.join("metadata.fb2.lua");

            if epub_metadata_path.exists() {
                let book_filename = format!("{}.epub", book_stem);
                match index.entry(book_filename.clone()) {
                    std::collections::hash_map::Entry::Occupied(_) => duplicates.push(book_filename),
                    std::collections::hash_map::Entry::Vacant(entry) => { entry.insert(epub_metadata_path); }
                }
            } else if fb2_metadata_path.exists() {
                let book_filename = format!("{}.fb2", book_stem);
                match index.entry(book_filename.clone()) {
                    std::collections::hash_map::Entry::Occupied(_) => duplicates.push(book_filename),
                    std::collections::hash_map::Entry::Vacant(entry) => { entry.insert(fb2_metadata_path); }
                }
            }
        }
    }
    if !duplicates.is_empty() { bail!("Duplicate books in docsettings: {:?}", duplicates); }
    Ok(index)
}

fn build_hashdocsettings_index(hashdocsettings_path: &PathBuf) -> Result<HashMap<String, PathBuf>> {
    let mut index: HashMap<String, PathBuf> = HashMap::new();
    info!("Scanning hashdocsettings folder: {:?}", hashdocsettings_path);

    for entry in walkdir::WalkDir::new(hashdocsettings_path).max_depth(3) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => { warn!("Failed to read entry: {}", e); continue; }
        };
        let path = entry.path();
        if path.is_dir()
            && let Some(dir_name) = path.file_name().and_then(|s| s.to_str())
            && let Some(hash) = dir_name.strip_suffix(".sdr")
        {
            if hash.len() == 32 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                let epub_metadata_path = path.join("metadata.epub.lua");
                if epub_metadata_path.exists() {
                    index.insert(hash.to_lowercase(), epub_metadata_path);
                } else if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str()
                            && name.starts_with("metadata.") && name.ends_with(".lua")
                        {
                            index.insert(hash.to_lowercase(), entry.path());
                            break;
                        }
                    }
                }
            }
        }
    }
    Ok(index)
}

pub struct Scanner {
    pub config: SiteConfig,
    metadata_location: MetadataLocation,
    docsettings_index: Option<HashMap<String, PathBuf>>,
    hashdocsettings_index: Option<HashMap<String, PathBuf>>,
    epub_parser: EpubParser,
    fb2_parser: Fb2Parser,
    comic_parser: ComicParser,
    mobi_parser: MobiParser,
    lua_parser: LuaParser,
}

impl Scanner {
    pub fn new(config: SiteConfig) -> Self {
        let metadata_location = config.metadata_location.clone();
        let docsettings_index = match &metadata_location {
            MetadataLocation::DocSettings(path) => build_docsettings_index(path).ok(),
            _ => None,
        };
        let hashdocsettings_index = match &metadata_location {
            MetadataLocation::HashDocSettings(path) => build_hashdocsettings_index(path).ok(),
            _ => None,
        };

        Self {
            config,
            metadata_location,
            docsettings_index,
            hashdocsettings_index,
            epub_parser: EpubParser::new(),
            fb2_parser: Fb2Parser::new(),
            comic_parser: ComicParser::new(),
            mobi_parser: MobiParser::new(),
            lua_parser: LuaParser::new(),
        }
    }

    pub async fn scan(&self) -> Result<Vec<LibraryItem>> {
        let (books, _) = scan_library(&self.config.library_paths, &self.metadata_location).await?;
        Ok(books)
    }

    async fn parse_book_info(&self, format: LibraryItemFormat, path: &Path) -> Result<BookInfo> {
        match format {
            LibraryItemFormat::Epub => self.epub_parser.parse(path).await,
            LibraryItemFormat::Fb2 => self.fb2_parser.parse(path).await,
            LibraryItemFormat::Cbz | LibraryItemFormat::Cbr => self.comic_parser.parse(path).await,
            LibraryItemFormat::Mobi => self.mobi_parser.parse(path).await,
        }
    }

    fn locate_metadata_path_and_md5(&self, path: &Path, format: LibraryItemFormat) -> (Option<PathBuf>, Option<String>) {
        let mut book_md5: Option<String> = None;
        let metadata_path = match &self.metadata_location {
            MetadataLocation::InBookFolder => {
                let book_stem = path.file_stem().unwrap().to_str().unwrap();
                let sdr_path = path.parent().unwrap().join(format!("{}.sdr", book_stem));
                let metadata_file = sdr_path.join(format.metadata_filename());
                metadata_file.exists().then_some(metadata_file)
            }
            MetadataLocation::DocSettings(_) => {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    self.docsettings_index.as_ref().and_then(|idx| idx.get(filename).cloned())
                } else { None }
            }
            MetadataLocation::HashDocSettings(_) => {
                match calculate_partial_md5(path) {
                    Ok(hash) => {
                        book_md5 = Some(hash.clone());
                        self.hashdocsettings_index.as_ref().and_then(|idx| idx.get(&hash.to_lowercase()).cloned())
                    }
                    Err(_) => None
                }
            }
        };
        (metadata_path, book_md5)
    }

    async fn parse_koreader_metadata(&self, metadata_path: Option<PathBuf>) -> Option<KoReaderMetadata> {
        let path = metadata_path?;
        self.lua_parser.parse(&path).await.ok()
    }

    fn collect_md5_for_item(&self, library_md5s: &mut HashSet<String>, path: &Path, koreader_metadata: &Option<KoReaderMetadata>, book_md5: &Option<String>) {
        if let Some(metadata) = koreader_metadata {
            if let Some(md5) = metadata.partial_md5_checksum.as_ref() {
                library_md5s.insert(md5.clone());
                return;
            }
        }
        if let Some(md5) = book_md5 {
            library_md5s.insert(md5.clone());
        } else if let Ok(md5) = calculate_partial_md5(path) {
            library_md5s.insert(md5);
        }
    }
}

pub async fn scan_library(
    library_paths: &[PathBuf],
    metadata_location: &MetadataLocation,
) -> Result<(Vec<LibraryItem>, HashSet<String>)> {
    
    // CORREÇÃO FINAL DOS TIPOS:
    // min_pages/min_time corrigidos para None (Option<u32>)
    // language corrigido para String::new() para satisfazer a struct
    let scanner = Scanner::new(SiteConfig {
        library_paths: library_paths.to_vec(),
        metadata_location: metadata_location.clone(),
        output_dir: PathBuf::new(),
        site_title: String::new(),
        include_unread: false,
        statistics_db_path: None,
        heatmap_scale_max: Some(0),
        // TimeConfig::new(Option<Tz>, u16)
        time_config: TimeConfig::new(None, 0), 
        min_pages_per_day: None, 
        min_time_per_day: None,  
        include_all_stats: false,
        is_internal_server: false,
        language: String::new(), 
    });

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().template("{spinner:.cyan} {msg}").unwrap());
    spinner.set_message("Scanning library...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let mut books = Vec::new();
    let mut library_md5s = HashSet::new();

    for library_path in library_paths {
        for entry in walkdir::WalkDir::new(library_path) {
            let entry = entry?;
            let path = entry.path();
            let format = match LibraryItemFormat::from_path(path) {
                Some(f) => f,
                None => continue,
            };
            let book_info = match scanner.parse_book_info(format, path).await {
                Ok(info) => info,
                Err(_) => continue,
            };
            let (metadata_path, book_md5) = scanner.locate_metadata_path_and_md5(path, format);
            let koreader_metadata = scanner.parse_koreader_metadata(metadata_path).await;
            
            // Lógica de coleta de MD5 integrada
            scanner.collect_md5_for_item(&mut library_md5s, path, &koreader_metadata, &book_md5);
            
            books.push(LibraryItem {
                id: generate_book_id(&book_info.title),
                book_info,
                koreader_metadata,
                file_path: path.to_path_buf(),
                format,
            });
            spinner.set_message(format!("Scanning library... {} items found", books.len()));
        }
    }
    spinner.finish_and_clear();
    Ok((books, library_md5s))
}