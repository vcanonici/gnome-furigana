//! Contextual morphological analysis with Lindera and the embedded `UniDic` (NINJAL).
//!
//! `UniDic` gives the reading Japanese readers expect in UI text (行 → ぎょう in
//! "行 1", where IPADIC said くだり), at the cost of a larger binary; it is mapped
//! lazily, so resident memory stays small.

use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

use crate::kana::to_hiragana;

// `UniDic` (unidic-mecab 2.1.2) feature columns, 0-based, after the surface form.
const PART_OF_SPEECH: usize = 0;
/// 語彙素読み: kana reading of the lexeme, without long-vowel marks (ヒョウカ).
const LEXEME_READING: usize = 6;
/// 発音形: pronunciation of this form, with long-vowel marks (ヒョーカ, 言っ → イッ).
const PRONUNCIATION: usize = 9;
/// 書字形基本形: dictionary form in the input's own spelling (切り替える).
const BASE_FORM: usize = 10;
const MISSING: &str = "*";
const LONG_VOWEL: char = 'ー';

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
    /// Loads the embedded `UniDic` dictionary.
    ///
    /// # Errors
    /// Returns an error if the embedded dictionary cannot be decoded.
    pub fn new() -> Result<Self, MorphologyError> {
        let dictionary = load_dictionary("embedded://unidic").map_err(|error| MorphologyError(error.to_string()))?;
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

/// Kana reading of this form. The pronunciation is used with each long-vowel mark
/// replaced by the lexeme reading's vowel at the same position (ヒョーカ → ヒョウカ).
/// For an unconjugated word that result must differ from the lexeme reading only by
/// sound changes (rendaku 日 ヒ → ビ, っ in 一本, ゜ in 本 → ポン); any other difference
/// is pronunciation spelling (言う ユー) and the lexeme reading wins.
fn surface_reading(
    surface: &str,
    base_form: Option<&str>,
    lexeme: Option<&str>,
    pronunciation: Option<&str>,
) -> String {
    let reading = match (lexeme, pronunciation) {
        (Some(lexeme), Some(pronunciation)) => {
            let reconciled = reconcile_long_vowels(pronunciation, lexeme);
            let is_unconjugated = base_form == Some(surface);
            if is_unconjugated && !differs_only_by_sound_changes(&reconciled, lexeme) {
                lexeme.to_owned()
            } else {
                reconciled
            }
        }
        (None, Some(reading)) | (Some(reading), None) => reading.to_owned(),
        (None, None) => String::new(),
    };
    to_hiragana(&reading)
}

/// Katakana without voicing marks: ガ → カ, バ/パ → ハ, ヴ → ウ.
fn unvoiced(character: char) -> char {
    let offset = match character {
        'ガ' | 'ギ' | 'グ' | 'ゲ' | 'ゴ' | 'ザ' | 'ジ' | 'ズ' | 'ゼ' | 'ゾ' | 'ダ' | 'ヂ' | 'ヅ' | 'デ' | 'ド'
        | 'バ' | 'ビ' | 'ブ' | 'ベ' | 'ボ' => 1,
        'パ' | 'ピ' | 'プ' | 'ペ' | 'ポ' => 2,
        'ヴ' => return 'ウ',
        _ => 0,
    };
    char::from_u32(character as u32 - offset).unwrap_or(character)
}

fn differs_only_by_sound_changes(actual: &str, lexeme: &str) -> bool {
    let (actual, lexeme): (Vec<char>, Vec<char>) = (actual.chars().collect(), lexeme.chars().collect());
    actual.len() == lexeme.len()
        && actual.iter().zip(&lexeme).all(|(&a, &l)| a == l || a == 'ッ' || unvoiced(a) == unvoiced(l))
}

fn reconcile_long_vowels(pronunciation: &str, lexeme: &str) -> String {
    let lexeme: Vec<char> = lexeme.chars().collect();
    pronunciation
        .chars()
        .enumerate()
        .map(|(index, character)| match lexeme.get(index) {
            Some(&replacement) if character == LONG_VOWEL && replacement != LONG_VOWEL => replacement,
            _ => character,
        })
        .collect()
}

fn morpheme_from_details(surface: String, details: &[String], start: usize) -> Morpheme {
    let end = start + surface.chars().count();
    let base_form = detail(details, BASE_FORM);
    Morpheme {
        reading: surface_reading(&surface, base_form, detail(details, LEXEME_READING), detail(details, PRONUNCIATION)),
        lemma: base_form.unwrap_or(&surface).to_owned(),
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

    fn readings(text: &str) -> Vec<(String, String)> {
        analyze(text).into_iter().map(|m| (m.surface, m.reading)).collect()
    }

    #[test]
    fn reads_power_off_in_context() {
        let words = analyze("電源を切る");
        let summary: Vec<(&str, &str, usize, usize)> =
            words.iter().map(|m| (m.surface.as_str(), m.reading.as_str(), m.start, m.end)).collect();
        assert_eq!(summary, vec![("電源", "でんげん", 0, 2), ("を", "を", 2, 3), ("切る", "きる", 3, 5)]);
    }

    #[test]
    fn reads_line_counter_and_long_vowels_correctly() {
        assert_eq!(readings("行 1")[0].1, "ぎょう");
        assert_eq!(readings("評価")[0].1, "ひょうか");
        assert_eq!(readings("今日")[0].1, "きょう");
    }

    #[test]
    fn sound_changes_follow_the_pronunciation() {
        let joined = |text: &str| readings(text).into_iter().map(|(_, r)| r).collect::<String>();
        // 本 after a number is ポン (handakuten); 言う is pronounced ユー but read いう.
        assert_eq!(readings("一本")[1].1, "ぽん");
        assert_eq!(readings("言う")[0].1, "いう");
        assert_eq!(joined("土曜"), "どよう");
    }

    #[test]
    fn conjugated_forms_keep_their_own_reading_and_dictionary_lemma() {
        let words = analyze("切り替えて");
        assert_eq!((words[0].surface.as_str(), words[0].reading.as_str()), ("切り替え", "きりかえ"));
        assert_eq!(words[0].lemma, "切り替える");
        assert_eq!(readings("言って")[0].1, "いっ");
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
