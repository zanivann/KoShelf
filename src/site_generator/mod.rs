//! Site generator module - orchestrates static site generation for KoShelf.

pub mod assets;
pub mod cache_manifest;
pub mod calendar;
pub mod library_pages;
pub mod recap;
pub mod statistics;
pub mod utils;

pub use cache_manifest::CacheManifestBuilder;

use crate::config::SiteConfig;
use crate::i18n::Translations;
use crate::koreader::{StatisticsCalculator, StatisticsParser, calculate_partial_md5};
use crate::library::scan_library;
use crate::models::{BookStatus, ContentType, LibraryItem, StatisticsData};
use crate::library::MetadataLocation; 
use crate::time_config::TimeConfig;
use anyhow::Result;
use log::{info, error, warn};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use utils::{NavContext, UiContext};

// --- FUNÇÃO GLOBAL DE GERAÇÃO (Usada pelo web.rs) ---

/// Helper function to trigger full site regeneration from Web Server API.
pub fn generate_site(library: &Vec<LibraryItem>, output_dir: &Path) -> Result<()> {
    // 1. Get current global language
    let lang = crate::i18n::get_global_locale();

    info!("Starting manual site regeneration. Language: {}, Items: {}, Output: {:?}", 
          lang, library.len(), output_dir);

    // 2. Reconstruct minimal configuration needed for generation.
    let config = SiteConfig {
        output_dir: output_dir.to_path_buf(),
        library_paths: vec![], // We won't scan again
        metadata_location: MetadataLocation::default(), 
        statistics_db_path: None, 
        language: lang,
        site_title: "KoShelf Library".to_string(),
        include_unread: true, 
        include_all_stats: false,
        heatmap_scale_max: None,
        min_pages_per_day: None,
        min_time_per_day: None,
        time_config: TimeConfig::new(None, 0), 
        is_internal_server: true,
    };

    // 3. Instantiate Generator
    let generator = SiteGenerator::new(config);

    // 4. Force Generation using a temporary runtime
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let result = rt.block_on(async {
        generator.generate_with_items(library.clone()).await
    });

    match &result {
        Ok(_) => info!("Regeneration completed successfully."),
        Err(e) => error!("Regeneration failed: {:?}", e),
    }

    result
}

// --- ESTRUTURAS PRINCIPAIS ---

#[derive(Debug)]
struct GenerationContext {
    all_items: Vec<LibraryItem>,
    books: Vec<LibraryItem>,
    comics: Vec<LibraryItem>,
    stats_data: Option<StatisticsData>,
    recap_latest_href: Option<String>,
    nav: NavContext,
}

pub struct SiteGenerator {
    config: SiteConfig,
    /// Cache manifest builder for PWA smart caching
    cache_manifest: Arc<CacheManifestBuilder>,
    /// Translations for i18n
    translations: Rc<Translations>,
}

impl std::ops::Deref for SiteGenerator {
    type Target = SiteConfig;
    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl SiteGenerator {
    pub fn new(config: SiteConfig) -> Self {
        let version = chrono::Local::now().to_rfc3339();
        let cache_manifest = Arc::new(CacheManifestBuilder::new(version));

        // Load translations for the configured language or global fallback
        let current_lang = if config.language != "en" { 
            config.language.clone() 
        } else {
            crate::i18n::get_global_locale() 
        };

        let translations = Rc::new(Translations::load(&current_lang).unwrap_or_else(|e| {
            warn!("Failed to load translations for '{}': {}. Fallback to en.", current_lang, e);
            Translations::load("en").expect("English translations must exist")
        }));

        Self {
            config,
            cache_manifest,
            translations,
        }
    }

    pub(crate) fn t(&self) -> Rc<Translations> {
        Rc::clone(&self.translations)
    }

    // Path constants
    pub(crate) fn books_dir(&self) -> PathBuf { self.output_dir.join("books") }
    pub(crate) fn comics_dir(&self) -> PathBuf { self.output_dir.join("comics") }
    pub(crate) fn calendar_dir(&self) -> PathBuf { self.output_dir.join("calendar") }
    pub(crate) fn statistics_dir(&self) -> PathBuf { self.output_dir.join("statistics") }
    pub(crate) fn recap_dir(&self) -> PathBuf { self.output_dir.join("recap") }
    pub(crate) fn assets_dir(&self) -> PathBuf { self.output_dir.join("assets") }
    pub(crate) fn covers_dir(&self) -> PathBuf { self.assets_dir().join("covers") }
    pub(crate) fn css_dir(&self) -> PathBuf { self.assets_dir().join("css") }
    pub(crate) fn js_dir(&self) -> PathBuf { self.assets_dir().join("js") }
    pub(crate) fn icons_dir(&self) -> PathBuf { self.assets_dir().join("icons") }
    pub(crate) fn json_dir(&self) -> PathBuf { self.assets_dir().join("json") }
    pub(crate) fn statistics_json_dir(&self) -> PathBuf { self.json_dir().join("statistics") }
    pub(crate) fn calendar_json_dir(&self) -> PathBuf { self.json_dir().join("calendar") }

    fn recap_latest_href(stats_data: Option<&StatisticsData>) -> Option<String> {
        let sd = stats_data?;
        let mut years: Vec<i32> = Vec::new();
        for b in &sd.books {
            if let Some(cs) = &b.completions {
                for c in &cs.entries {
                    if c.end_date.len() >= 4 {
                        if let Ok(y) = c.end_date[0..4].parse::<i32>() {
                            if !years.contains(&y) { years.push(y); }
                        }
                    }
                }
            }
        }
        if years.is_empty() { None } else {
            years.sort_by(|a, b| b.cmp(a));
            Some(format!("/recap/{}/", years[0]))
        }
    }

    async fn build_context_from_items(&self, items: Vec<LibraryItem>) -> Result<GenerationContext> {
        let all_items = items;
        let books: Vec<_> = all_items.iter().filter(|b| b.is_book()).cloned().collect();
        let comics: Vec<_> = all_items.iter().filter(|b| b.is_comic()).cloned().collect();

        let has_books = !books.is_empty();
        let has_comics = !comics.is_empty();
        let stats_data = None; 
        let recap_latest_href = Self::recap_latest_href(stats_data.as_ref());

        let nav = NavContext {
            has_books,
            has_comics,
            stats_at_root: stats_data.is_some() && all_items.is_empty(),
        };

        Ok(GenerationContext { all_items, books, comics, stats_data, recap_latest_href, nav })
    }

    async fn build_generation_context(&self) -> Result<GenerationContext> {
        let (all_items, library_md5s) = if !self.library_paths.is_empty() {
            scan_library(&self.library_paths, &self.metadata_location).await?
        } else {
            (Vec::new(), HashSet::new())
        };

        let all_items: Vec<_> = all_items.into_iter().filter(|item| {
            if item.koreader_metadata.is_some() { return true; }
            self.include_unread && item.status() == BookStatus::Unknown
        }).collect();

        let books: Vec<_> = all_items.iter().filter(|b| b.is_book()).cloned().collect();
        let comics: Vec<_> = all_items.iter().filter(|b| b.is_comic()).cloned().collect();

        let has_books = !books.is_empty();
        let has_comics = !comics.is_empty();

        let mut stats_data = if let Some(ref stats_path) = self.statistics_db_path {
            if stats_path.exists() {
                let mut data = StatisticsParser::parse(stats_path)?;
                if self.min_pages_per_day.is_some() || self.min_time_per_day.is_some() {
                    StatisticsCalculator::filter_stats(&mut data, &self.time_config, self.min_pages_per_day, self.min_time_per_day);
                }
                if !self.include_all_stats && !all_items.is_empty() {
                    StatisticsCalculator::filter_to_library(&mut data, &library_md5s);
                }
                StatisticsCalculator::populate_completions(&mut data, &self.time_config);
                Some(data)
            } else {
                info!("Statistics database not found: {:?}", stats_path);
                None
            }
        } else {
            None
        };

        let recap_latest_href = Self::recap_latest_href(stats_data.as_ref());
        let nav = NavContext {
            has_books,
            has_comics,
            stats_at_root: stats_data.is_some() && all_items.is_empty(),
        };

        if let Some(ref mut sd) = stats_data {
            let mut md5_to_content_type: HashMap<String, ContentType> = HashMap::new();
            for item in &all_items {
                let md5 = item.koreader_metadata.as_ref()
                    .and_then(|m| m.partial_md5_checksum.as_ref())
                    .cloned()
                    .or_else(|| calculate_partial_md5(&item.file_path).ok());
                if let Some(md5) = md5 {
                    md5_to_content_type.insert(md5, item.content_type());
                }
            }
            sd.tag_content_types(&md5_to_content_type);
        }

        Ok(GenerationContext { all_items, books, comics, stats_data, recap_latest_href, nav })
    }

    pub async fn generate(&self) -> Result<()> {
        let ctx = self.build_generation_context().await?;
        self.generate_from_context(ctx).await
    }

    pub async fn generate_with_items(&self, items: Vec<LibraryItem>) -> Result<()> {
        let ctx = self.build_context_from_items(items).await?;
        self.generate_from_context(ctx).await
    }

    async fn generate_from_context(&self, mut ctx: GenerationContext) -> Result<()> {
        info!("Generating static site in: {:?}", self.output_dir);
        
        let ui = UiContext {
            recap_latest_href: ctx.recap_latest_href.clone(),
            nav: ctx.nav,
        };

        self.create_directories(&ctx.all_items, &ctx.stats_data).await?;
        self.copy_static_assets(&ctx.all_items, &ctx.stats_data).await?;
        self.generate_covers(&ctx.all_items).await?;

        self.cleanup_stale_books(&ctx.books)?;
        self.cleanup_stale_comics(&ctx.comics)?;
        self.cleanup_stale_covers(&ctx.all_items)?;

        self.generate_book_pages(&ctx.books, &mut ctx.stats_data, &ui).await?;
        self.generate_comic_pages(&ctx.comics, &mut ctx.stats_data, &ui).await?;

        if ctx.nav.has_books { self.generate_book_list(&ctx.books, &ui).await?; }
        if ctx.nav.has_comics { self.generate_comic_list(&ctx.comics, !ctx.nav.has_books, &ui).await?; }

        if let Some(ref mut stats_data) = ctx.stats_data {
            self.generate_statistics_page(stats_data, ctx.all_items.is_empty(), &ui).await?;
            self.generate_calendar_page(stats_data, &ctx.all_items, &ui).await?;
            self.generate_recap_pages(stats_data, &ctx.all_items, ctx.nav).await?;
        }

        self.cache_manifest.write(self.output_dir.join("cache-manifest.json"))?;
        info!("Static site generation completed!");
        Ok(())
    }
}