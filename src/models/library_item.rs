use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::koreader_metadata::KoReaderMetadata;

/// Content type classification (broad category)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Book,
    Comic,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::Book => write!(f, "book"),
            ContentType::Comic => write!(f, "comic"),
        }
    }
}

/// Supported library item formats (ebooks + comics)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LibraryItemFormat {
    Epub,
    Fb2,
    Mobi,
    Cbz,
    Cbr,
}

impl LibraryItemFormat {
    pub fn from_path(path: &Path) -> Option<Self> {
        let filename = path.file_name()?.to_str()?.to_lowercase();

        if filename.ends_with(".fb2.zip") {
            return Some(Self::Fb2);
        }

        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "epub" => Some(Self::Epub),
            "fb2" => Some(Self::Fb2),
            "mobi" => Some(Self::Mobi),
            "cbz" => Some(Self::Cbz),
            #[cfg(not(windows))]
            "cbr" => Some(Self::Cbr),
            _ => None,
        }
    }

    pub fn metadata_filename(&self) -> &'static str {
        match self {
            Self::Epub => "metadata.epub.lua",
            Self::Fb2 => "metadata.fb2.lua",
            Self::Mobi => "metadata.mobi.lua",
            Self::Cbz => "metadata.cbz.lua",
            Self::Cbr => "metadata.cbr.lua",
        }
    }

    pub fn is_metadata_file(filename: &str) -> bool {
        matches!(
            filename,
            "metadata.epub.lua"
                | "metadata.fb2.lua"
                | "metadata.mobi.lua"
                | "metadata.cbz.lua"
                | "metadata.cbr.lua"
        )
    }

    pub fn content_type(&self) -> ContentType {
        match self {
            Self::Epub | Self::Fb2 | Self::Mobi => ContentType::Book,
            Self::Cbz | Self::Cbr => ContentType::Comic,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identifier {
    pub scheme: String,
    pub value: String,
}

impl Identifier {
    pub fn new(scheme: String, value: String) -> Self {
        Self { scheme, value }
    }

    pub fn display_scheme(&self) -> String {
        match self.scheme.to_lowercase().as_str() {
            "isbn" => "ISBN".to_string(),
            "google" => "Google Books".to_string(),
            "amazon" | "asin" | "mobi-asin" => "Amazon".to_string(),
            "goodreads" => "Goodreads".to_string(),
            "doi" => "DOI".to_string(),
            "kobo" => "Kobo".to_string(),
            "oclc" => "WorldCat".to_string(),
            "lccn" => "Library of Congress".to_string(),
            "hardcover" | "hardcover-slug" => "Hardcover".to_string(),
            "hardcover-edition" => "Hardcover Edition".to_string(),
            _ => self.scheme.clone(),
        }
    }

    pub fn url(&self) -> Option<String> {
        match self.scheme.to_lowercase().as_str() {
            "isbn" => Some(format!("https://www.worldcat.org/isbn/{}", self.value)),
            "google" => Some(format!("https://books.google.com/books?id={}", self.value)),
            "amazon" | "asin" | "mobi-asin" => {
                Some(format!("https://www.amazon.com/dp/{}", self.value))
            }
            "goodreads" => Some(format!(
                "https://www.goodreads.com/book/show/{}",
                self.value
            )),
            "doi" => Some(format!("https://doi.org/{}", self.value)),
            "kobo" => Some(format!("https://www.kobo.com/ebook/{}", self.value)),
            "oclc" => Some(format!("https://www.worldcat.org/oclc/{}", self.value)),
            "hardcover" | "hardcover-edition" => {
                Some(format!("https://hardcover.app/books/{}", self.value))
            }
            _ => None,
        }
    }

    pub fn is_linkable(&self) -> bool {
        self.url().is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryItem {
    pub id: String,
    pub book_info: BookInfo,
    pub koreader_metadata: Option<KoReaderMetadata>,
    pub file_path: PathBuf,
    pub format: LibraryItemFormat,
}

impl LibraryItem {
    pub fn status(&self) -> BookStatus {
        self.koreader_metadata
            .as_ref()
            .and_then(|m| m.summary.as_ref())
            .map(|s| s.status.clone())
            .unwrap_or(BookStatus::Unknown)
    }

    pub fn rating(&self) -> Option<u32> {
        self.koreader_metadata
            .as_ref()
            .and_then(|m| m.summary.as_ref())
            .and_then(|s| s.rating)
    }

    pub fn star_display(&self) -> [bool; 5] {
        let rating = self.rating().unwrap_or(0);
        [
            rating >= 1,
            rating >= 2,
            rating >= 3,
            rating >= 4,
            rating >= 5,
        ]
    }

    pub fn review_note(&self) -> Option<&String> {
        self.koreader_metadata
            .as_ref()
            .and_then(|m| m.summary.as_ref())
            .and_then(|s| s.note.as_ref())
    }

    pub fn progress_percentage(&self) -> Option<f64> {
        self.koreader_metadata
            .as_ref()
            .and_then(|m| m.percent_finished)
    }

    pub fn progress_percentage_display(&self) -> u32 {
        self.progress_percentage()
            .map(|p| (p * 100.0).round() as u32)
            .unwrap_or(0)
    }

    pub fn annotations(&self) -> &[super::koreader_metadata::Annotation] {
        self.koreader_metadata
            .as_ref()
            .map(|m| m.annotations.as_slice())
            .unwrap_or(&[])
    }

    pub fn annotation_count(&self) -> usize {
        self.koreader_metadata
            .as_ref()
            .map(|m| m.annotations.len())
            .unwrap_or(0)
    }

    pub fn bookmark_count(&self) -> usize {
        self.koreader_metadata
            .as_ref()
            .map(|m| m.annotations.iter().filter(|a| a.is_bookmark()).count())
            .unwrap_or(0)
    }

    pub fn highlight_count(&self) -> usize {
        self.koreader_metadata
            .as_ref()
            .map(|m| m.annotations.iter().filter(|a| a.is_highlight()).count())
            .unwrap_or(0)
    }

    pub fn doc_pages(&self) -> Option<u32> {
        self.koreader_metadata
            .as_ref()
            .and_then(|m| m.doc_pages)
            .or(self.book_info.pages)
    }

    pub fn note_count(&self) -> usize {
        self.koreader_metadata
            .as_ref()
            .and_then(|m| m.stats.as_ref())
            .and_then(|s| s.notes)
            .map(|n| n as usize)
            .unwrap_or(0)
    }

    pub fn language(&self) -> Option<&String> {
        self.book_info.language.as_ref().or_else(|| {
            self.koreader_metadata
                .as_ref()
                .and_then(|m| m.text_lang.as_ref())
        })
    }

    pub fn publisher(&self) -> Option<&String> {
        self.book_info.publisher.as_ref()
    }

    pub fn identifiers(&self) -> Vec<Identifier> {
        let mut result: Vec<Identifier> = Vec::new();
        let mut dedupe_keys: HashSet<(String, String)> = HashSet::new();

        for id in self.get_normalized_hardcover_identifiers() {
            let key = (id.scheme.to_lowercase(), id.value.clone());
            if dedupe_keys.insert(key) {
                result.push(id);
            }
        }

        for id in &self.book_info.identifiers {
            let scheme_lc = id.scheme.to_lowercase();
            if scheme_lc == "hardcover"
                || scheme_lc == "hardcover-slug"
                || scheme_lc == "hardcover-edition"
            {
                continue;
            }
            let key = (scheme_lc, id.value.clone());
            if dedupe_keys.insert(key) {
                result.push(id.clone());
            }
        }

        result
            .into_iter()
            .filter(|id| id.display_scheme() != id.scheme)
            .collect()
    }

    pub fn subjects(&self) -> &Vec<String> {
        &self.book_info.subjects
    }

    pub fn subjects_display(&self) -> Option<String> {
        if self.book_info.subjects.is_empty() {
            None
        } else {
            Some(self.book_info.subjects.join(", "))
        }
    }

    fn get_normalized_hardcover_identifiers(&self) -> Vec<Identifier> {
        let mut out: Vec<Identifier> = Vec::new();

        let slug = self
            .book_info
            .identifiers
            .iter()
            .find(|id| {
                let s = id.scheme.to_lowercase();
                s == "hardcover" || s == "hardcover-slug"
            })
            .map(|id| id.value.clone());

        if let Some(slug_val) = slug {
            out.push(Identifier::new("hardcover".to_string(), slug_val.clone()));
            for id in &self.book_info.identifiers {
                if id.scheme.eq_ignore_ascii_case("hardcover-edition") {
                    out.push(Identifier::new(
                        "hardcover-edition".to_string(),
                        format!("{}/editions/{}", slug_val, id.value),
                    ));
                }
            }
        }

        out
    }

    pub fn series(&self) -> Option<&String> {
        self.book_info.series.as_ref()
    }

    pub fn series_number(&self) -> Option<&String> {
        self.book_info.series_number.as_ref()
    }

    pub fn series_display(&self) -> Option<String> {
        match (self.series(), self.series_number()) {
            (Some(series), Some(number)) => Some(format!("{} #{}", series, number)),
            (Some(series), None) => Some(series.clone()),
            _ => None,
        }
    }

    pub fn content_type(&self) -> ContentType {
        self.format.content_type()
    }

    pub fn is_comic(&self) -> bool {
        self.content_type() == ContentType::Comic
    }

    pub fn is_book(&self) -> bool {
        self.content_type() == ContentType::Book
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookInfo {
    pub title: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub language: Option<String>,
    pub publisher: Option<String>,
    pub identifiers: Vec<Identifier>,
    pub subjects: Vec<String>,
    pub series: Option<String>,
    pub series_number: Option<String>,
    pub pages: Option<u32>,
    pub cover_data: Option<Vec<u8>>,
    pub cover_mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BookStatus {
    Reading,
    Complete,
    Abandoned,
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for BookStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BookStatus::Reading => write!(f, "reading"),
            BookStatus::Complete => write!(f, "complete"),
            BookStatus::Abandoned => write!(f, "abandoned"),
            BookStatus::Unknown => write!(f, "unknown"),
        }
    }
}