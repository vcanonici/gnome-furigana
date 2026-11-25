//! Turns a UI string into furigana segments and per-word meanings.

use std::num::NonZeroUsize;
use std::sync::Arc;

use lru::LruCache;

use crate::alignment::{Segment, align};
use crate::counters::{apply_counter_readings, counter_meaning};
use crate::glossary::Glossary;
use crate::inline_readings::{ReadingHint, render_segments, strip_inline_readings};
use crate::kana::has_kanji;
use crate::morphology::{Morpheme, MorphologicalAnalyzer, MorphologyError};
use crate::protocol::{Analysis, Token};

/// Longest accepted input, in code points. UI labels are far shorter.
pub const MAX_TEXT_CHARS: usize = 512;
const CACHE_ENTRIES: NonZeroUsize = NonZeroUsize::new(4096).unwrap();
/// IPADIC splits compounds (拡張|機能); adjacent kanji words are merged when the
/// glossary knows the compound, trying the longest span first. Spans with kana words
/// (particles, katakana) are whole expressions: they stay word-level so every word
/// keeps its own hover, and the expression meaning is reported as `phrase`.
const MAX_COMPOUND_WORDS: usize = 4;

/// Errors returned to callers.
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    /// Input exceeds [`MAX_TEXT_CHARS`].
    #[error("text longer than {MAX_TEXT_CHARS} characters")]
    TooLong,
    /// The morphological analyzer failed.
    #[error(transparent)]
    Morphology(#[from] MorphologyError),
}

/// Façade over morphology + alignment + glossary with an in-memory LRU.
/// Nothing is persisted: analysed UI text never touches the disk.
pub struct Analyzer {
    morphology: MorphologicalAnalyzer,
    glossary: Glossary,
    cache: LruCache<String, Arc<Analysis>>,
}

impl Analyzer {
    /// Creates an analyzer over an already loaded glossary.
    #[must_use]
    pub fn new(morphology: MorphologicalAnalyzer, glossary: Glossary) -> Self {
        Self { morphology, glossary, cache: LruCache::new(CACHE_ENTRIES) }
    }

    /// The glossary in use.
    #[must_use]
    pub fn glossary(&self) -> &Glossary {
        &self.glossary
    }

    /// Analyses one UI string; repeated strings are served from the cache.
    ///
    /// # Errors
    /// Rejects text longer than [`MAX_TEXT_CHARS`] or a failing analyzer.
    pub fn analyze(&mut self, text: &str) -> Result<Arc<Analysis>, AnalysisError> {
        if text.chars().count() > MAX_TEXT_CHARS {
            return Err(AnalysisError::TooLong);
        }
        if let Some(cached) = self.cache.get(text) {
            return Ok(Arc::clone(cached));
        }
        let analysis = Arc::new(self.analyze_uncached(text)?);
        self.cache.put(text.to_owned(), Arc::clone(&analysis));
        Ok(analysis)
    }

    fn analyze_uncached(&self, text: &str) -> Result<Analysis, AnalysisError> {
        let stripped = strip_inline_readings(text);
        let plain = stripped.plain.as_str();
        if !has_kanji(plain) {
            return Ok(Analysis {
                text: text.to_owned(),
                plain_text: plain.to_owned(),
                reading_hints_applied: 0,
                tokens: Vec::new(),
                phrase: None,
                display_inline: plain.to_owned(),
            });
        }
        let words = merge_compounds(&apply_counter_readings(&self.morphology.analyze(plain)?), &self.glossary);
        let mut tokens: Vec<Token> = words.into_iter().map(|word| self.token(word)).collect();
        let reading_hints_applied = tokens.iter_mut().map(|token| apply_reading_hints(token, &stripped.hints)).sum();
        Ok(Analysis {
            text: text.to_owned(),
            plain_text: plain.to_owned(),
            reading_hints_applied,
            phrase: self.glossary.lookup_curated(trim_ellipsis(plain)),
            display_inline: render_inline_text(plain, &tokens),
            tokens,
        })
    }

    fn token(&self, mut word: Morpheme) -> Token {
        // Reviewed readings (curated glossary, user overrides) beat the dictionary for
        // unconjugated words: 空 in UI text is から, not うつろ.
        if let Some(reading) = self.reviewed_reading(&word) {
            word.reading = reading;
        }
        let annotate = has_kanji(&word.surface);
        let segments = if annotate { align(&word.surface, &word.reading) } else { align(&word.surface, "") };
        let is_read = segments.iter().any(|segment| segment.reading.is_some());
        let meaning = annotate
            .then(|| self.glossary.lookup(&word.lemma, Some(&word.reading)))
            .flatten()
            .or_else(|| counter_meaning(&word.surface, &word.reading));
        Token {
            reading: is_read.then(|| word.reading.clone()),
            part_of_speech: meaning
                .as_ref()
                .and_then(|entry| entry.part_of_speech.clone())
                .unwrap_or_else(|| part_of_speech_pt(&word.part_of_speech).to_owned()),
            surface: word.surface,
            start: word.start,
            end: word.end,
            segments,
            lemma: word.lemma,
            meaning,
        }
    }

    fn reviewed_reading(&self, word: &Morpheme) -> Option<String> {
        if word.surface != word.lemma || !has_kanji(&word.surface) {
            return None;
        }
        self.glossary.lookup_curated(&word.surface)?.reading
    }
}

/// Replaces segment readings with readings the input already carried for exactly the
/// same kanji run (they come from our own catalogs, i.e. the same aligner). Returns how
/// many were applied.
fn apply_reading_hints(token: &mut Token, hints: &[ReadingHint]) -> usize {
    let mut applied = 0;
    let mut start = token.start;
    for segment in &mut token.segments {
        let end = start + segment.text.chars().count();
        if let Some(hint) = hints.iter().find(|hint| hint.start == start && hint.end == end) {
            if segment.reading.is_some() {
                segment.reading = Some(hint.reading.clone());
                applied += 1;
            }
        }
        start = end;
    }
    if applied > 0 {
        token.reading = Some(token.segments.iter().map(segment_reading).collect());
    }
    applied
}

fn segment_reading(segment: &Segment) -> String {
    segment.reading.clone().unwrap_or_else(|| crate::kana::to_hiragana(&segment.text))
}

/// Inline rendering of the whole text: words get `漢字(かんじ)`, everything between
/// them (spaces, punctuation the analyzer skipped) is copied unchanged.
fn render_inline_text(text: &str, tokens: &[Token]) -> String {
    let characters: Vec<char> = text.chars().collect();
    let mut rendered = String::with_capacity(text.len() * 2);
    let mut cursor = 0;
    for token in tokens {
        rendered.extend(characters.get(cursor..token.start).unwrap_or_default());
        rendered.push_str(&render_segments(&token.segments));
        cursor = token.end;
    }
    rendered.extend(characters.get(cursor..).unwrap_or_default());
    rendered
}

fn trim_ellipsis(text: &str) -> &str {
    text.trim().trim_end_matches(['…', '.']).trim()
}

/// Merges runs of adjacent words whose concatenated surface is a curated term.
fn merge_compounds(words: &[Morpheme], glossary: &Glossary) -> Vec<Morpheme> {
    let mut merged = Vec::with_capacity(words.len());
    let mut index = 0;
    while index < words.len() {
        let span = (2..=MAX_COMPOUND_WORDS.min(words.len() - index))
            .rev()
            .find(|&length| is_curated_compound(&words[index..index + length], glossary))
            .unwrap_or(1);
        merged.push(if span == 1 { words[index].clone() } else { join_words(&words[index..index + span], glossary) });
        index += span;
    }
    merged
}

fn is_curated_compound(words: &[Morpheme], glossary: &Glossary) -> bool {
    words.iter().all(|word| has_kanji(&word.surface)) && glossary.lookup_curated(&concat_surfaces(words)).is_some()
}

fn concat_surfaces(words: &[Morpheme]) -> String {
    words.iter().map(|word| word.surface.as_str()).collect()
}

/// Joins words into one compound. The curated reading wins over the concatenation:
/// a dictionary that splits 土曜|日 cannot apply the rendaku of 土曜日 (どようび).
fn join_words(words: &[Morpheme], glossary: &Glossary) -> Morpheme {
    let surface = concat_surfaces(words);
    let curated_reading = glossary.lookup_curated(&surface).and_then(|entry| entry.reading);
    Morpheme {
        reading: curated_reading.unwrap_or_else(|| words.iter().map(|word| word.reading.as_str()).collect()),
        lemma: surface.clone(),
        part_of_speech: words.last().map(|word| word.part_of_speech.clone()).unwrap_or_default(),
        start: words.first().map_or(0, |word| word.start),
        end: words.last().map_or(0, |word| word.end),
        surface,
    }
}

fn part_of_speech_pt(japanese: &str) -> &str {
    match japanese {
        "名詞" => "substantivo",
        "動詞" => "verbo",
        "形容詞" => "adjetivo",
        "副詞" => "advérbio",
        "助詞" => "partícula",
        "助動詞" => "verbo auxiliar",
        "接頭詞" => "prefixo",
        "連体詞" => "determinante",
        "記号" => "símbolo",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glossary::Source;

    fn analyzer() -> Analyzer {
        let directory = std::env::temp_dir().join("gnome-furigana-test-empty");
        Analyzer::new(MorphologicalAnalyzer::new().unwrap(), Glossary::load(&directory, &directory).unwrap())
    }

    #[test]
    fn settings_has_reading_and_portuguese_meaning() {
        let analysis = analyzer().analyze("設定").unwrap();
        let meaning = analysis.tokens[0].meaning.as_ref().unwrap();
        assert_eq!(analysis.tokens[0].reading.as_deref(), Some("せってい"));
        assert_eq!((meaning.glosses[0].as_str(), meaning.source), ("configurações", Source::Glossary));
    }

    #[test]
    fn power_off_expression_is_segmented_and_translated_as_a_whole() {
        let analysis = analyzer().analyze("電源を切る").unwrap();
        assert_eq!(analysis.display_inline, "電源(でんげん)を切(き)る");
        assert_eq!(analysis.phrase.as_ref().unwrap().glosses[0], "desligar (o computador)");
    }

    #[test]
    fn compounds_split_by_the_dictionary_are_merged_when_curated() {
        let analysis = analyzer().analyze("拡張機能を無効化").unwrap();
        let surfaces: Vec<&str> = analysis.tokens.iter().map(|token| token.surface.as_str()).collect();
        assert_eq!(surfaces[0], "拡張機能");
        assert_eq!(analysis.tokens[0].reading.as_deref(), Some("かくちょうきのう"));
    }

    #[test]
    fn expressions_stay_word_level_with_a_phrase_meaning() {
        let analysis = analyzer().analyze("ユーザーを切り替え…").unwrap();
        let surfaces: Vec<&str> = analysis.tokens.iter().map(|token| token.surface.as_str()).collect();
        assert_eq!(surfaces, vec!["ユーザー", "を", "切り替え", "…"]);
        assert_eq!(analysis.phrase.as_ref().unwrap().glosses[0], "trocar de usuário");
    }

    #[test]
    fn inline_rendering_keeps_text_between_words() {
        let analysis = analyzer().analyze("11月24日 16:59").unwrap();
        assert_eq!(analysis.display_inline, "11月(じゅういちがつ)24日(にじゅうよっか) 16:59");
    }

    #[test]
    fn reviewed_readings_win_for_single_words() {
        let analysis = analyzer().analyze("パレットが空なので").unwrap();
        assert!(analysis.display_inline.contains("空(から)"), "{}", analysis.display_inline);
    }

    #[test]
    fn curated_compounds_carry_their_reviewed_reading() {
        let analysis = analyzer().analyze("土曜日").unwrap();
        assert_eq!(analysis.display_inline, "土曜日(どようび)");
    }

    #[test]
    fn already_annotated_text_is_not_annotated_twice() {
        let analysis = analyzer().analyze("電源(でんげん)管理(かんり)").unwrap();
        assert_eq!(analysis.plain_text, "電源管理");
        assert_eq!(analysis.display_inline, "電源(でんげん)管理(かんり)");
        assert_eq!(analysis.reading_hints_applied, 2);
    }

    #[test]
    fn mnemonic_parentheses_survive() {
        let analysis = analyzer().analyze("固定ワークスペース(_F)").unwrap();
        assert_eq!(analysis.display_inline, "固定(こてい)ワークスペース(_F)");
    }

    #[test]
    fn text_without_kanji_is_untouched_and_long_text_is_rejected() {
        let mut analyzer = analyzer();
        assert_eq!(analyzer.analyze("Bluetooth").unwrap().tokens, Vec::new());
        assert!(matches!(analyzer.analyze(&"設".repeat(MAX_TEXT_CHARS + 1)), Err(AnalysisError::TooLong)));
    }

    #[test]
    fn unknown_words_have_no_invented_meaning() {
        let analysis = analyzer().analyze("𠮷野家").unwrap();
        assert!(analysis.tokens.iter().all(|token| token.meaning.is_none()));
    }
}
