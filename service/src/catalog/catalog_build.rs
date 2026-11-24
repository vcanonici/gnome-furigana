//! Builds the `ja@furigana` gettext catalogs from the system's Japanese catalogs.
//!
//! Each output holds only the header and the messages that actually gained readings;
//! glibc takes every other message from the next language in `LANGUAGE` (`ja`), so
//! the original catalogs are never copied, modified or shadowed beyond what changed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::message_annotation::{annotate_message, is_date_format};
use super::mo_file::{self, Message};
use crate::analysis::Analyzer;

/// Japanese catalogs, highest priority first (Ubuntu language packs override `locale`).
pub const DEFAULT_SOURCES: [&str; 2] =
    ["/usr/share/locale-langpack/ja/LC_MESSAGES", "/usr/share/locale/ja/LC_MESSAGES"];
/// Where glibc finds `LANGUAGE=ja@furigana` on Ubuntu.
pub const DEFAULT_OUTPUT: &str = "/usr/share/locale-langpack/ja@furigana/LC_MESSAGES";
/// Domains annotated live by the Shell extension (ruby above kanji) instead.
pub const DEFAULT_EXCLUDED: [&str; 2] = ["gnome-shell", "mutter"];
const INVENTORY_FILE: &str = ".furigana-inventory.json";
const MAX_MESSAGE_BYTES: usize = 8 * 1024;

/// What to build.
pub struct BuildOptions {
    /// Source directories, highest priority first.
    pub sources: Vec<PathBuf>,
    /// Output `LC_MESSAGES` directory.
    pub output: PathBuf,
    /// Domains (file stems) to skip.
    pub excluded: BTreeSet<String>,
    /// Report without writing.
    pub dry_run: bool,
}

/// Outcome of a build, printed as JSON by the CLI.
#[derive(Debug, Default, Serialize)]
pub struct BuildReport {
    /// Catalogs (re)written.
    pub built: usize,
    /// Catalogs whose source was unchanged since the last build.
    pub unchanged: usize,
    /// Catalogs with nothing to annotate.
    pub empty: usize,
    /// Catalogs in a format we do not rewrite (e.g. `.mo` revision 1); glibc keeps
    /// using the original `ja` catalog for them.
    pub unsupported: Vec<String>,
    /// Catalogs that could not be read, as `domain: reason`.
    pub failed: Vec<String>,
    /// Stale outputs removed because their source disappeared.
    pub removed: usize,
    /// Messages that gained readings in the catalogs built now.
    pub messages_annotated: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Inventory {
    builder: String,
    sources: BTreeMap<String, String>,
}

/// Errors that stop a whole build (single bad catalogs are reported, not fatal).
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Creating or writing the output directory failed.
    #[error("{path}: {source}")]
    Io {
        /// Path involved.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> BuildError + '_ {
    move |source| BuildError::Io { path: path.to_owned(), source }
}

/// Domain → source file, honouring source priority.
fn discover_sources(options: &BuildOptions) -> BTreeMap<String, PathBuf> {
    let mut domains = BTreeMap::new();
    for directory in &options.sources {
        let Ok(entries) = std::fs::read_dir(directory) else { continue };
        for path in entries.flatten().map(|entry| entry.path()) {
            let is_catalog = path.extension().is_some_and(|extension| extension == "mo");
            let domain = path.file_stem().map(|stem| stem.to_string_lossy().into_owned());
            if let (true, Some(domain)) = (is_catalog, domain) {
                if !options.excluded.contains(&domain) {
                    domains.entry(domain).or_insert(path);
                }
            }
        }
    }
    domains
}

fn builder_version() -> String {
    format!("{}+{}", env!("CARGO_PKG_VERSION"), crate::glossary::curated_fingerprint())
}

fn read_inventory(output: &Path) -> Inventory {
    std::fs::read_to_string(output.join(INVENTORY_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(|inventory: &Inventory| inventory.builder == builder_version())
        .unwrap_or_default()
}

/// Annotates every plural form of one message; header and date formats are skipped.
fn annotate_entry(analyzer: &mut Analyzer, message: &Message) -> Option<Message> {
    let original = std::str::from_utf8(&message.original).ok()?;
    let translation = std::str::from_utf8(&message.translation).ok()?;
    if original.is_empty() || message.translation.len() > MAX_MESSAGE_BYTES || is_date_format(original) {
        return None;
    }
    let forms: Vec<Option<String>> = translation.split('\0').map(|form| annotate_message(analyzer, form)).collect();
    if forms.iter().all(Option::is_none) {
        return None;
    }
    let joined =
        translation.split('\0').zip(forms).map(|(form, annotated)| annotated.unwrap_or_else(|| form.to_owned()));
    Some(Message {
        original: message.original.clone(),
        translation: joined.collect::<Vec<_>>().join("\0").into_bytes(),
    })
}

/// Returns the annotated catalog (header + changed messages) or `None` when nothing changed.
fn annotate_catalog(analyzer: &mut Analyzer, bytes: &[u8]) -> Result<Option<Vec<u8>>, mo_file::MoError> {
    let messages = mo_file::parse(bytes)?;
    let annotated: Vec<Message> = messages.iter().filter_map(|message| annotate_entry(analyzer, message)).collect();
    if annotated.is_empty() {
        return Ok(None);
    }
    // The header carries charset and Plural-Forms, which plural messages need.
    let header = messages.into_iter().filter(|message| message.original.is_empty());
    Ok(Some(mo_file::serialize(&header.chain(annotated).collect::<Vec<_>>())))
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), BuildError> {
    let temporary = path.with_extension("mo.tmp");
    std::fs::write(&temporary, bytes).map_err(io_error(&temporary))?;
    std::fs::rename(&temporary, path).map_err(io_error(path))
}

/// Builds or refreshes all catalogs; unchanged sources are skipped.
///
/// # Errors
/// Fails only when the output directory cannot be created or written.
pub fn build_catalogs(analyzer: &mut Analyzer, options: &BuildOptions) -> Result<BuildReport, BuildError> {
    let (mut report, previous) = (BuildReport::default(), read_inventory(&options.output));
    let mut inventory = Inventory { builder: builder_version(), sources: BTreeMap::new() };
    if !options.dry_run {
        std::fs::create_dir_all(&options.output).map_err(io_error(&options.output))?;
    }
    let sources = discover_sources(options);
    for (domain, path) in &sources {
        let Ok(bytes) = std::fs::read(path) else {
            report.failed.push(format!("{domain}: unreadable"));
            continue;
        };
        let digest = crate::content_digest::sha256_hex(&bytes, 32);
        let target = options.output.join(format!("{domain}.mo"));
        if previous.sources.get(domain) == Some(&digest) {
            report.unchanged += 1;
        } else {
            let failures_before = report.failed.len();
            build_one(analyzer, options, &bytes, &target, domain, &mut report)?;
            // Failed catalogs are not recorded, so the next build retries them.
            if report.failed.len() > failures_before {
                continue;
            }
        }
        inventory.sources.insert(domain.clone(), digest);
    }
    report.removed = remove_stale(options, &previous, &sources)?;
    if !options.dry_run {
        let text = serde_json::to_string_pretty(&inventory).unwrap_or_default();
        write_atomically(&options.output.join(INVENTORY_FILE), text.as_bytes())?;
    }
    Ok(report)
}

fn build_one(
    analyzer: &mut Analyzer,
    options: &BuildOptions,
    bytes: &[u8],
    target: &Path,
    domain: &str,
    report: &mut BuildReport,
) -> Result<(), BuildError> {
    match annotate_catalog(analyzer, bytes) {
        Err(error @ mo_file::MoError::UnsupportedRevision(_)) => report.unsupported.push(format!("{domain}: {error}")),
        Err(error) => report.failed.push(format!("{domain}: {error}")),
        Ok(None) => {
            report.empty += 1;
            if !options.dry_run && target.exists() {
                std::fs::remove_file(target).map_err(io_error(target))?;
            }
        }
        Ok(Some(catalog)) => {
            report.built += 1;
            report.messages_annotated +=
                mo_file::parse(&catalog).map_or(0, |messages| messages.len().saturating_sub(1));
            if !options.dry_run {
                write_atomically(target, &catalog)?;
            }
        }
    }
    Ok(())
}

/// Deletes outputs whose source catalog no longer exists (e.g. package removed).
fn remove_stale(
    options: &BuildOptions,
    previous: &Inventory,
    sources: &BTreeMap<String, PathBuf>,
) -> Result<usize, BuildError> {
    let stale: Vec<&String> = previous.sources.keys().filter(|domain| !sources.contains_key(*domain)).collect();
    if !options.dry_run {
        for domain in &stale {
            let path = options.output.join(format!("{domain}.mo"));
            if path.exists() {
                std::fs::remove_file(&path).map_err(io_error(&path))?;
            }
        }
    }
    Ok(stale.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glossary::Glossary;
    use crate::morphology::MorphologicalAnalyzer;

    fn analyzer() -> Analyzer {
        let directory = std::env::temp_dir().join("gnome-furigana-test-empty");
        Analyzer::new(MorphologicalAnalyzer::new().unwrap(), Glossary::load(&directory, &directory).unwrap())
    }

    fn catalog(messages: &[(&str, &str)]) -> Vec<u8> {
        let messages: Vec<Message> = messages
            .iter()
            .map(|(o, t)| Message { original: o.as_bytes().to_vec(), translation: t.as_bytes().to_vec() })
            .collect();
        mo_file::serialize(&messages)
    }

    fn options(source: &Path, output: &Path) -> BuildOptions {
        BuildOptions {
            sources: vec![source.to_owned()],
            output: output.to_owned(),
            excluded: BTreeSet::from(["gnome-shell".into()]),
            dry_run: false,
        }
    }

    #[test]
    fn builds_only_changed_messages_and_skips_unchanged_sources() {
        let root = tempfile::tempdir().unwrap();
        let (source, output) = (root.path().join("ja"), root.path().join("out"));
        std::fs::create_dir_all(&source).unwrap();
        let header = "Content-Type: text/plain; charset=UTF-8\n";
        std::fs::write(
            source.join("app.mo"),
            catalog(&[("", header), ("Power", "電源管理"), ("Wi-Fi", "Wi-Fi"), ("%Y-%m-%d", "%Y年%m月%d日")]),
        )
        .unwrap();
        std::fs::write(source.join("gnome-shell.mo"), catalog(&[("Settings", "設定")])).unwrap();
        let mut analyzer = analyzer();
        let report = build_catalogs(&mut analyzer, &options(&source, &output)).unwrap();
        assert_eq!((report.built, report.messages_annotated), (1, 1));
        let built = mo_file::parse(&std::fs::read(output.join("app.mo")).unwrap()).unwrap();
        let translations: Vec<String> =
            built.iter().map(|m| String::from_utf8(m.translation.clone()).unwrap()).collect();
        assert_eq!(translations, vec![header.to_owned(), "電源(でんげん)管理(かんり)".to_owned()]);
        assert!(!output.join("gnome-shell.mo").exists());
        let again = build_catalogs(&mut analyzer, &options(&source, &output)).unwrap();
        assert_eq!((again.built, again.unchanged), (0, 1));
    }

    #[test]
    fn stale_outputs_are_removed_when_the_source_disappears() {
        let root = tempfile::tempdir().unwrap();
        let (source, output) = (root.path().join("ja"), root.path().join("out"));
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("app.mo"), catalog(&[("Power", "電源管理")])).unwrap();
        let mut analyzer = analyzer();
        build_catalogs(&mut analyzer, &options(&source, &output)).unwrap();
        std::fs::remove_file(source.join("app.mo")).unwrap();
        let report = build_catalogs(&mut analyzer, &options(&source, &output)).unwrap();
        assert_eq!(report.removed, 1);
        assert!(!output.join("app.mo").exists());
    }
}
