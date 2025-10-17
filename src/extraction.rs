use crate::format;

use anyhow::{Context, Result};
use lopdf::Document;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Extract text from a PDF at `pdf_path`, optionally caching the extracted
/// text in `cache_dir`. `start` and `end` are 1-based page indices (inclusive).
///
/// Behavior:
/// - If `cache_dir` is Some and a cache file exists for the given PDF (and page range),
///   the cached text is returned.
/// - Otherwise, the PDF is parsed using `lopdf::Document::load` and `extract_text`,
///   the resulting text is optionally written to the cache file, and returned.
///
/// The cache filename is derived from the PDF filename and page range:
/// e.g., `document.pdf` pages 1-5 -> `document_1-5.txt`.
pub fn extract_pdf_to_string(
    pdf_path: &Path,
    cache_dir: Option<&Path>,
    start: Option<u32>,
    end: Option<u32>,
) -> Result<String> {
    // If a cache directory was supplied, ensure it exists and compute a candidate cache path.
    let cache_path: Option<PathBuf> = cache_dir.map(|dir| {
        let stem = pdf_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document");
        let mut name = stem.to_string();
        if let Some(s) = start {
            name.push('_');
            name.push_str(&s.to_string());
        }
        if let Some(e) = end {
            name.push('-');
            name.push_str(&e.to_string());
        }
        dir.join(format!("{}.txt", sanitize_filename(&name)))
    });

    // If cached file exists, read and return it
    if let Some(ref cp) = cache_path {
        if cp.exists() {
            let cached = fs::read_to_string(cp).unwrap_or_default();
            if !cached.is_empty() {
                return Ok(cached);
            }
        }
    }

    // Load the PDF document
    let doc = Document::load(pdf_path).context("Failed to load PDF document")?;

    // Determine pages to extract
    let pages = doc.get_pages();
    let mut sorted_pages: Vec<u32> = pages.keys().cloned().collect();
    sorted_pages.sort();

    let start_page = start.unwrap_or(1);
    let end_page = end.unwrap_or(*sorted_pages.last().unwrap_or(&u32::MAX));

    let page_numbers_to_extract: Vec<u32> = sorted_pages
        .into_iter()
        .filter(|&p| p >= start_page && p <= end_page)
        .collect();

    if page_numbers_to_extract.is_empty() {
        return Ok(String::new());
    }

    // Extract text
    let text = doc
        .extract_text(&page_numbers_to_extract)
        .context("Failed to extract text from PDF")?;

    // Optionally write to cache
    if let Some(ref cp) = cache_path {
        if let Some(parent) = cp.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(mut f) = fs::File::create(cp) {
            let _ = f.write_all(text.as_bytes());
        }
    }

    Ok(text)
}

/// A small helper to sanitize parts of filenames for cache paths.
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => '_',
            other => other,
        })
        .collect()
}

/// Convenience helper that extracts and immediately runs formatting on the
/// extracted text using the `format` module (if present). This returns cleaned
/// text ready for further processing.
///
/// This function does not depend on the CLI `ParsedArgs`; callers can pass
/// the PDF path, optional cache directory, and optional start/end pages.
pub fn extract_and_format(
    pdf_path: &Path,
    cache_dir: Option<&Path>,
    start: Option<u32>,
    end: Option<u32>,
) -> Result<String> {
    let raw = extract_pdf_to_string(pdf_path, cache_dir, start, end)?;
    // Use `format::clean_text` if available; otherwise do a lightweight fallback.
    let cleaned = match format::clean_text(&raw) {
        s if !s.is_empty() => s,
        _ => lightweight_clean(&raw),
    };
    Ok(cleaned)
}

/// Lightweight fallback cleaning (minimal) if the `format` module would produce nothing.
fn lightweight_clean(s: &str) -> String {
    let t = s.replace('\r', "").replace('\x0C', " ").replace("-\n", "");
    // Collapse runs of whitespace to single spaces and preserve paragraph breaks
    let mut out = String::new();
    let mut last_was_blank = false;
    for line in t.lines() {
        let trimmed = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if trimmed.is_empty() {
            if !last_was_blank {
                out.push('\n');
            }
            last_was_blank = true;
        } else {
            if !out.is_empty() && !last_was_blank {
                out.push(' ');
            }
            out.push_str(&trimmed);
            last_was_blank = false;
        }
    }
    out
}
