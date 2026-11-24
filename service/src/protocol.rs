//! JSON shapes returned over D-Bus (interface `io.github.vcanonici.Furigana1`).
//!
//! Offsets are Unicode code points, end-exclusive — the same unit as
//! `Clutter.Text` positions, so the extension can use them without conversion.

use serde::Serialize;

use crate::alignment::Segment;
use crate::glossary::Entry;

/// One analysed word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Token {
    /// Characters as they appear in the input.
    pub surface: String,
    /// Hiragana reading when the word has kanji and a usable reading.
    pub reading: Option<String>,
    /// First code point.
    pub start: usize,
    /// Code point after the word.
    pub end: usize,
    /// Furigana segments; concatenated `text` equals `surface`.
    pub segments: Vec<Segment>,
    /// Dictionary form used for lookup.
    pub lemma: String,
    /// Part of speech in pt-BR.
    pub part_of_speech: String,
    /// Meaning, or `None` when no source knows the word (never invented).
    pub meaning: Option<Entry>,
}

/// Result of `Analyze`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Analysis {
    /// The input text, unchanged.
    pub text: String,
    /// Words in order; empty when the text has no kanji.
    pub tokens: Vec<Token>,
    /// Curated meaning of the whole string when it is a known UI expression.
    pub phrase: Option<Entry>,
    /// Inline fallback rendering, e.g. `電源(でんげん)を切(き)る`.
    pub display_inline: String,
}

/// Result of `Health`.
#[derive(Debug, Clone, Serialize)]
pub struct Health {
    /// Service version.
    pub version: &'static str,
    /// D-Bus interface name.
    pub interface: &'static str,
    /// Seconds since start.
    pub uptime_seconds: u64,
    /// Method calls served.
    pub calls: u64,
    /// Entries in the curated glossary.
    pub glossary_terms: usize,
    /// Entries in the user's overrides.
    pub override_terms: usize,
    /// Whether the `JMdict` index is available.
    pub jmdict: bool,
}
