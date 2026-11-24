//! The inline furigana format `漢字(かんじ)`, in both directions.
//!
//! Annotated gettext catalogs and the Shell's inline fallback both use this module,
//! so text that already carries readings (e.g. an app name translated through a
//! `ja@furigana` catalog) is recognised instead of being annotated twice.

use crate::alignment::Segment;
use crate::kana::is_kanji;

const OPEN: char = '(';
const CLOSE: char = ')';

/// A reading found in already-annotated text, positioned on the stripped text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingHint {
    /// First code point of the kanji run in the stripped text.
    pub start: usize,
    /// Code point after the kanji run.
    pub end: usize,
    /// Hiragana reading of the run.
    pub reading: String,
}

/// Text with its inline readings removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrippedText {
    /// The text without `(かな)` annotations.
    pub plain: String,
    /// The removed readings, in order.
    pub hints: Vec<ReadingHint>,
}

/// Renders segments as `食(た)べる`.
#[must_use]
pub fn render_segments(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(|segment| match &segment.reading {
            Some(reading) => format!("{}{OPEN}{reading}{CLOSE}", segment.text),
            None => segment.text.clone(),
        })
        .collect()
}

fn is_hiragana(character: char) -> bool {
    matches!(character as u32, 0x3041..=0x309F) || character == 'ー'
}

/// Length in chars of a `(ひらがな)` group starting at `characters[0]`, if any.
fn reading_group_length(characters: &[char]) -> Option<usize> {
    if characters.first() != Some(&OPEN) {
        return None;
    }
    let inner = characters[1..].iter().take_while(|c| is_hiragana(**c)).count();
    (inner > 0 && characters.get(inner + 1) == Some(&CLOSE)).then_some(inner + 2)
}

/// Removes `(ひらがな)` groups that directly follow a kanji run and returns them as hints.
/// Anything else in parentheses (mnemonics like `(_F)`, katakana, Latin) is kept.
#[must_use]
pub fn strip_inline_readings(text: &str) -> StrippedText {
    let characters: Vec<char> = text.chars().collect();
    let mut plain = String::with_capacity(text.len());
    let (mut hints, mut plain_length, mut run_start, mut index) = (Vec::new(), 0, None, 0);
    while index < characters.len() {
        let character = characters[index];
        if let (Some(start), Some(length)) = (run_start, reading_group_length(&characters[index..])) {
            let reading = characters[index + 1..index + length - 1].iter().collect();
            hints.push(ReadingHint { start, end: plain_length, reading });
            (run_start, index) = (None, index + length);
            continue;
        }
        run_start = if is_kanji(character) { run_start.or(Some(plain_length)) } else { None };
        plain.push(character);
        (plain_length, index) = (plain_length + 1, index + 1);
    }
    StrippedText { plain, hints }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_readings_and_keeps_mnemonics() {
        let stripped = strip_inline_readings("固定(こてい)ワークスペース(_F)");
        assert_eq!(stripped.plain, "固定ワークスペース(_F)");
        assert_eq!(stripped.hints, vec![ReadingHint { start: 0, end: 2, reading: "こてい".into() }]);
    }

    #[test]
    fn positions_refer_to_the_stripped_text() {
        let stripped = strip_inline_readings("電源(でんげん)を切(き)る");
        assert_eq!(stripped.plain, "電源を切る");
        let spans: Vec<(usize, usize)> = stripped.hints.iter().map(|h| (h.start, h.end)).collect();
        assert_eq!(spans, vec![(0, 2), (3, 4)]);
    }

    #[test]
    fn parentheses_without_preceding_kanji_or_with_non_hiragana_stay() {
        for text in ["(かな)", "設定(Settings)", "ネット(ねっと)", "設定()"] {
            assert_eq!(strip_inline_readings(text).plain, text);
        }
    }

    #[test]
    fn render_and_strip_round_trip() {
        let segments = vec![
            Segment { text: "食".into(), reading: Some("た".into()) },
            Segment { text: "べる".into(), reading: None },
        ];
        let rendered = render_segments(&segments);
        assert_eq!(rendered, "食(た)べる");
        assert_eq!(strip_inline_readings(&rendered).plain, "食べる");
    }
}
