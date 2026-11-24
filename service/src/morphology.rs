//! Contextual morphological analysis with Lindera and the embedded IPADIC.

use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

use crate::kana::to_hiragana;

// IPADIC feature columns (0-based) after the surface form.
const PART_OF_SPEECH: usize = 0;
const BASE_FORM: usize = 6;
const READING: usize = 7;
const MISSING: &str = "*";

/// One word of the input. Offsets are Unicode code points, end-exclusive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Morpheme {
    /// Characters as they appear in the input.
    pub surface: String,
    /// Hiragana reading in context, empty when the dictionary does not know the word.
    pub reading: String,
    /// Dictionary (lemma) form, used for meaning lookup.
    pub lemma: String,
    /// Coarse part of speech in Japanese (名詞, 動詞, …).
    pub part_of_speech: String,
    /// First code point of the word.
    pub start: usize,
    /// Code point after the word.
    pub end: usize,
}

/// Errors produced while analysing text.
#[derive(Debug, thiserror::Error)]
#[error("morphological analysis failed: {0}")]
pub struct MorphologyError(String);

/// Wraps the Lindera segmenter; the embedded dictionary is loaded once.
pub struct MorphologicalAnalyzer {
    segmenter: Segmenter,
}

impl MorphologicalAnalyzer {
    /// Loads the embedded IPADIC dictionary.
    ///
    /// # Errors
    /// Returns an error if the embedded dictionary cannot be decoded.
    pub fn new() -> Result<Self, MorphologyError> {
        let dictionary = load_dictionary("embedded://ipadic").map_err(|error| MorphologyError(error.to_string()))?;
        Ok(Self { segmenter: Segmenter::new(Mode::Normal, dictionary, None) })
    }

    /// Splits text into morphemes with hiragana readings and code-point offsets.
    ///
    /// # Errors
    /// Returns an error if the segmenter rejects the input.
    pub fn analyze(&self, text: &str) -> Result<Vec<Morpheme>, MorphologyError> {
        let mut tokens =
            self.segmenter.segment(Cow::Borrowed(text)).map_err(|error| MorphologyError(error.to_string()))?;
        let mut morphemes = Vec::with_capacity(tokens.len());
        for token in &mut tokens {
            let details: Vec<String> = token.details().iter().map(ToString::to_string).collect();
            // Lindera drops whitespace, so offsets come from its byte positions, never from
            // summing surfaces.
            let start = text.get(..token.byte_start).map_or(0, |prefix| prefix.chars().count());
            morphemes.push(morpheme_from_details(token.surface.as_ref().to_owned(), &details, start));
        }
        Ok(morphemes)
    }
}

fn detail(details: &[String], index: usize) -> Option<&str> {
    details.get(index).map(String::as_str).filter(|value| *value != MISSING)
}

fn morpheme_from_details(surface: String, details: &[String], start: usize) -> Morpheme {
    let end = start + surface.chars().count();
    Morpheme {
        reading: detail(details, READING).map(to_hiragana).unwrap_or_default(),
        lemma: detail(details, BASE_FORM).unwrap_or(&surface).to_owned(),
        part_of_speech: detail(details, PART_OF_SPEECH).unwrap_or_default().to_owned(),
        surface,
        start,
        end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(text: &str) -> Vec<Morpheme> {
        MorphologicalAnalyzer::new().unwrap().analyze(text).unwrap()
    }

    #[test]
    fn reads_power_off_in_context() {
        let words = analyze("電源を切る");
        let summary: Vec<(&str, &str, usize, usize)> =
            words.iter().map(|m| (m.surface.as_str(), m.reading.as_str(), m.start, m.end)).collect();
        assert_eq!(summary, vec![("電源", "でんげん", 0, 2), ("を", "を", 2, 3), ("切る", "きる", 3, 5)]);
    }

    #[test]
    fn offsets_survive_dropped_whitespace() {
        let last = analyze("11月24日 16:59 設定").pop().unwrap();
        assert_eq!((last.surface.as_str(), last.start, last.end), ("設定", 13, 15));
    }

    #[test]
    fn offsets_count_code_points_outside_the_bmp() {
        let last = analyze("𠮷野家に行く").pop().unwrap();
        assert_eq!((last.surface.as_str(), last.start, last.end), ("行く", 4, 6));
    }
}
