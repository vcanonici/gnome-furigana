//! Aligns a word's reading to its kanji runs so okurigana stays unannotated.
//!
//! 食べる + たべる → [食(た), べる]. If the split is ambiguous the whole word is
//! annotated as one span (correct, only less granular). If the reading is unusable,
//! no annotation is produced: a wrong reading is worse than none.

use serde::Serialize;

use crate::kana::{is_kana, is_kanji, to_hiragana};

/// A piece of a word; `reading` is `None` for kana or symbols that need no furigana.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Segment {
    /// Original characters of this piece.
    pub text: String,
    /// Hiragana reading for kanji pieces.
    pub reading: Option<String>,
}

impl Segment {
    fn plain(text: &str) -> Self {
        Self { text: text.to_owned(), reading: None }
    }

    fn annotated(text: &str, reading: &str) -> Self {
        Self { text: text.to_owned(), reading: Some(reading.to_owned()) }
    }
}

struct Run {
    text: String,
    is_kanji: bool,
}

fn split_runs(surface: &str) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for character in surface.chars() {
        let kanji = is_kanji(character);
        match runs.last_mut() {
            Some(run) if run.is_kanji == kanji => run.text.push(character),
            _ => runs.push(Run { text: character.to_string(), is_kanji: kanji }),
        }
    }
    runs
}

/// Assigns reading characters to runs. Kana runs must match literally; each kanji
/// run takes at least one character. `shortest` picks the leftmost-shortest split,
/// otherwise the leftmost-longest one; comparing both detects ambiguity.
fn assign(runs: &[Run], reading: &[char], shortest: bool) -> Option<Vec<String>> {
    let Some((run, rest)) = runs.split_first() else {
        return reading.is_empty().then(Vec::new);
    };
    if !run.is_kanji {
        let kana: Vec<char> = to_hiragana(&run.text).chars().collect();
        let tail = reading.strip_prefix(kana.as_slice())?;
        return assign(rest, tail, shortest).map(|parts| prepend(String::from_iter(kana), parts));
    }
    let mut lengths: Vec<usize> = (1..=reading.len()).collect();
    if !shortest {
        lengths.reverse();
    }
    lengths.into_iter().find_map(|length| {
        let parts = assign(rest, &reading[length..], shortest)?;
        Some(prepend(reading[..length].iter().collect(), parts))
    })
}

fn prepend(first: String, mut rest: Vec<String>) -> Vec<String> {
    rest.insert(0, first);
    rest
}

fn is_usable_reading(surface: &str, reading: &str) -> bool {
    !reading.is_empty() && reading.chars().all(is_kana) && reading != to_hiragana(surface)
}

/// Returns furigana segments for one word, or one unannotated segment.
#[must_use]
pub fn align(surface: &str, reading: &str) -> Vec<Segment> {
    let hiragana = to_hiragana(reading);
    if !surface.chars().any(is_kanji) || !is_usable_reading(surface, &hiragana) {
        return vec![Segment::plain(surface)];
    }
    let runs = split_runs(surface);
    if runs.iter().any(|run| !run.is_kanji && !run.text.chars().all(is_kana)) {
        return vec![Segment::annotated(surface, &hiragana)];
    }
    let characters: Vec<char> = hiragana.chars().collect();
    let shortest = assign(&runs, &characters, true);
    match shortest {
        Some(parts) if Some(&parts) == assign(&runs, &characters, false).as_ref() => runs
            .iter()
            .zip(parts)
            .map(
                |(run, part)| {
                    if run.is_kanji { Segment::annotated(&run.text, &part) } else { Segment::plain(&run.text) }
                },
            )
            .collect(),
        _ => vec![Segment::annotated(surface, &hiragana)],
    }
}

/// Renders segments in the parenthesised fallback form 食(た)べる.
#[must_use]
pub fn render_inline(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(|segment| match &segment.reading {
            Some(reading) => format!("{}({reading})", segment.text),
            None => segment.text.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inline(surface: &str, reading: &str) -> String {
        render_inline(&align(surface, reading))
    }

    #[test]
    fn aligns_spec_examples() {
        assert_eq!(inline("食べる", "タベル"), "食(た)べる");
        assert_eq!(inline("行く", "イク"), "行(い)く");
        assert_eq!(inline("今日", "キョウ"), "今日(きょう)");
        assert_eq!(inline("日本語", "ニホンゴ"), "日本語(にほんご)");
        assert_eq!(inline("銀行", "ギンコウ"), "銀行(ぎんこう)");
        assert_eq!(inline("電源", "デンゲン"), "電源(でんげん)");
    }

    #[test]
    fn aligns_interleaved_okurigana_and_katakana() {
        assert_eq!(inline("切り替え", "キリカエ"), "切(き)り替(か)え");
        assert_eq!(inline("機内モード", "キナイモード"), "機内(きない)モード");
        assert_eq!(inline("時々", "トキドキ"), "時々(ときどき)");
    }

    #[test]
    fn leaves_kana_and_unknown_words_unannotated() {
        assert_eq!(align("オフ", "オフ"), vec![Segment::plain("オフ")]);
        assert_eq!(align("𠮷", "*"), vec![Segment::plain("𠮷")]);
    }

    #[test]
    fn impossible_split_falls_back_to_whole_word() {
        assert_eq!(align("食べる", "クウ"), vec![Segment::annotated("食べる", "くう")]);
    }
}
