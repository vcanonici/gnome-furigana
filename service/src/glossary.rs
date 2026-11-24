//! Meaning lookup in priority order: user overrides → curated UI glossary (pt-BR)
//! → `JMdict` (English). Nothing is invented: unknown terms return `None`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::jmdict_index::JmdictIndex;

const CURATED_GLOSSARY: &str = include_str!("../../data/ui-glossary-pt_BR.tsv");

/// Where a meaning came from; shown to the user as provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    /// The user's own `overrides.tsv`.
    Override,
    /// The curated GNOME UI glossary shipped with the project.
    Glossary,
    /// The `JMdict` dictionary (English glosses).
    Jmdict,
}

/// A meaning with its provenance; `language` is a BCP 47 tag of the glosses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// Dictionary form of the term.
    pub term: String,
    /// Hiragana reading, when known.
    pub reading: Option<String>,
    /// Up to three senses, most relevant first.
    pub glosses: Vec<String>,
    /// Part of speech label.
    pub part_of_speech: Option<String>,
    /// Provenance of the meaning.
    pub source: Source,
    /// Language of `glosses` (`pt-BR` or `en`).
    pub language: &'static str,
}

/// A malformed glossary line.
#[derive(Debug, thiserror::Error)]
#[error("{origin}:{line}: expected `term<TAB>reading<TAB>gloss;…[<TAB>pos]`")]
pub struct GlossaryParseError {
    origin: String,
    line: usize,
}

/// Parses TSV glossary text; blank lines and lines starting with `#` are ignored.
/// The first occurrence of a term wins.
///
/// # Errors
/// Returns the first line that does not have at least three columns.
pub fn parse_glossary(text: &str, origin: &str, source: Source) -> Result<HashMap<String, Entry>, GlossaryParseError> {
    let mut entries = HashMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let columns: Vec<&str> = line.split('\t').map(str::trim).collect();
        let [term, reading, glosses, rest @ ..] = columns.as_slice() else {
            return Err(GlossaryParseError { origin: origin.to_owned(), line: index + 1 });
        };
        entries.entry((*term).to_owned()).or_insert_with(|| Entry {
            term: (*term).to_owned(),
            reading: (!reading.is_empty()).then(|| (*reading).to_owned()),
            glosses: glosses.split(';').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect(),
            part_of_speech: rest.first().filter(|s| !s.is_empty()).map(|s| (*s).to_owned()),
            source,
            language: "pt-BR",
        });
    }
    Ok(entries)
}

/// In-memory curated tables plus the optional on-disk `JMdict` index.
pub struct Glossary {
    overrides: HashMap<String, Entry>,
    curated: HashMap<String, Entry>,
    jmdict: Option<JmdictIndex>,
}

/// Errors while loading glossary sources.
#[derive(Debug, thiserror::Error)]
pub enum GlossaryLoadError {
    /// A TSV file is malformed.
    #[error(transparent)]
    Parse(#[from] GlossaryParseError),
    /// The overrides file exists but cannot be read.
    #[error("cannot read {path}: {source}")]
    Read {
        /// File that failed.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
}

impl Glossary {
    /// Loads the embedded curated glossary, `overrides.tsv` and `jmdict.sqlite` when present.
    ///
    /// # Errors
    /// Fails on malformed TSV or an unreadable overrides file; a missing file is not an error.
    pub fn load(config_dir: &Path, data_dir: &Path) -> Result<Self, GlossaryLoadError> {
        let overrides_path = config_dir.join("overrides.tsv");
        let overrides = match std::fs::read_to_string(&overrides_path) {
            Ok(text) => parse_glossary(&text, "overrides.tsv", Source::Override)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
            Err(source) => return Err(GlossaryLoadError::Read { path: overrides_path, source }),
        };
        let curated = parse_glossary(CURATED_GLOSSARY, "ui-glossary-pt_BR.tsv", Source::Glossary)?;
        Ok(Self { overrides, curated, jmdict: JmdictIndex::open(&data_dir.join("jmdict.sqlite")) })
    }

    /// Builds a glossary from explicit tables (tests only).
    #[cfg(test)]
    #[must_use]
    pub fn from_tables(overrides: HashMap<String, Entry>, curated: HashMap<String, Entry>) -> Self {
        Self { overrides, curated, jmdict: None }
    }

    /// Returns the best entry for a dictionary-form term.
    #[must_use]
    pub fn lookup(&self, term: &str, reading: Option<&str>) -> Option<Entry> {
        self.lookup_curated(term).or_else(|| self.jmdict.as_ref()?.lookup(term, reading))
    }

    /// Returns an entry only from overrides or the curated glossary (no dictionary fallback).
    #[must_use]
    pub fn lookup_curated(&self, term: &str) -> Option<Entry> {
        self.overrides.get(term).or_else(|| self.curated.get(term)).cloned()
    }

    /// Counts of loaded entries: (overrides, curated, jmdict available).
    #[must_use]
    pub fn stats(&self) -> (usize, usize, bool) {
        (self.overrides.len(), self.curated.len(), self.jmdict.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_glossary_parses_and_knows_settings() {
        let curated = parse_glossary(CURATED_GLOSSARY, "embedded", Source::Glossary).unwrap();
        assert_eq!(curated["設定"].glosses[0], "configurações");
        assert_eq!(curated["設定"].reading.as_deref(), Some("せってい"));
    }

    #[test]
    fn override_wins_over_curated() {
        let overrides = parse_glossary("設定\tせってい\tajustes", "o", Source::Override).unwrap();
        let curated = parse_glossary(CURATED_GLOSSARY, "c", Source::Glossary).unwrap();
        let entry = Glossary::from_tables(overrides, curated).lookup("設定", None).unwrap();
        assert_eq!((entry.glosses[0].as_str(), entry.source), ("ajustes", Source::Override));
    }

    #[test]
    fn malformed_line_reports_origin_and_line() {
        let error = parse_glossary("# c\n設定\tせってい", "x.tsv", Source::Override).unwrap_err();
        assert!(error.to_string().starts_with("x.tsv:2:"));
    }
}
