//! Readings of numbers followed by a date or counter kanji.
//!
//! IPADIC reads a lone 月 as つき ("moon") and every 日 as にち, so the clock's
//! `11月24日` would get wrong furigana. A number and its counter are merged into one
//! word whose reading comes from explicit tables: day-of-month readings are irregular
//! (1日 ついたち, 20日 はつか, 24日 にじゅうよっか) and must not be computed.

use crate::morphology::Morpheme;

/// じゅういちがつ etc. for months 1–12.
const MONTHS: [&str; 12] = [
    "いちがつ",
    "にがつ",
    "さんがつ",
    "しがつ",
    "ごがつ",
    "ろくがつ",
    "しちがつ",
    "はちがつ",
    "くがつ",
    "じゅうがつ",
    "じゅういちがつ",
    "じゅうにがつ",
];

/// Day-of-month readings 1–31.
const DAYS: [&str; 31] = [
    "ついたち",
    "ふつか",
    "みっか",
    "よっか",
    "いつか",
    "むいか",
    "なのか",
    "ようか",
    "ここのか",
    "とおか",
    "じゅういちにち",
    "じゅうににち",
    "じゅうさんにち",
    "じゅうよっか",
    "じゅうごにち",
    "じゅうろくにち",
    "じゅうしちにち",
    "じゅうはちにち",
    "じゅうくにち",
    "はつか",
    "にじゅういちにち",
    "にじゅうににち",
    "にじゅうさんにち",
    "にじゅうよっか",
    "にじゅうごにち",
    "にじゅうろくにち",
    "にじゅうしちにち",
    "にじゅうはちにち",
    "にじゅうくにち",
    "さんじゅうにち",
    "さんじゅういちにち",
];

/// Counters whose reading does not depend on the number (no sound changes).
const REGULAR_COUNTERS: [(&str, &str); 4] = [("年", "ねん"), ("時", "じ"), ("秒", "びょう"), ("週", "しゅう")];

fn parse_number(text: &str) -> Option<usize> {
    // Full-width digits appear in some catalogs; normalise them to ASCII.
    let ascii: String = text
        .chars()
        .map(|c| if ('０'..='９').contains(&c) { char::from_u32(c as u32 - 0xFEE0).unwrap_or(c) } else { c })
        .collect();
    (!ascii.is_empty() && ascii.chars().all(|c| c.is_ascii_digit())).then(|| ascii.parse().ok()).flatten()
}

fn counter_reading(number: usize, counter: &str) -> Option<String> {
    match counter {
        "月" => MONTHS.get(number.checked_sub(1)?).map(|r| (*r).to_owned()),
        "日" => DAYS.get(number.checked_sub(1)?).map(|r| (*r).to_owned()),
        _ => None,
    }
}

/// Joins consecutive digit tokens (full-width digits arrive one per token).
fn join_number(digits: &[Morpheme]) -> Option<Morpheme> {
    let (first, last) = (digits.first()?, digits.last()?);
    let surface: String = digits.iter().map(|word| word.surface.as_str()).collect();
    Some(Morpheme { lemma: surface.clone(), surface, end: last.end, ..first.clone() })
}

fn is_number(word: &Morpheme) -> bool {
    parse_number(&word.surface).is_some()
}

fn merge(number: &Morpheme, counter: &Morpheme, reading: String) -> Morpheme {
    let surface = format!("{}{}", number.surface, counter.surface);
    Morpheme {
        lemma: surface.clone(),
        surface,
        reading,
        part_of_speech: counter.part_of_speech.clone(),
        start: number.start,
        end: counter.end,
    }
}

/// Fixes the reading of a counter after a number, merging month/day expressions.
fn resolve_pair(number: &Morpheme, counter: &Morpheme) -> Option<Vec<Morpheme>> {
    let value = parse_number(&number.surface)?;
    if let Some(reading) = counter_reading(value, &counter.surface) {
        return Some(vec![merge(number, counter, reading)]);
    }
    let (_, reading) = REGULAR_COUNTERS.iter().find(|(kanji, _)| *kanji == counter.surface)?;
    Some(vec![number.clone(), Morpheme { reading: (*reading).to_owned(), ..counter.clone() }])
}

/// Applies counter readings to a morpheme sequence.
#[must_use]
pub fn apply_counter_readings(words: &[Morpheme]) -> Vec<Morpheme> {
    let mut result = Vec::with_capacity(words.len());
    let mut index = 0;
    while index < words.len() {
        let digits_end = index + words[index..].iter().take_while(|word| is_number(word)).count();
        let resolved = (digits_end > index)
            .then(|| join_number(&words[index..digits_end]))
            .flatten()
            .zip(words.get(digits_end))
            .and_then(|(number, counter)| resolve_pair(&number, counter));
        if let Some(resolved) = resolved {
            result.extend(resolved);
            index = digits_end + 1;
        } else {
            result.push(words[index].clone());
            index += 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morphology::MorphologicalAnalyzer;

    fn readings(text: &str) -> Vec<(String, String)> {
        let words = MorphologicalAnalyzer::new().unwrap().analyze(text).unwrap();
        apply_counter_readings(&words).into_iter().map(|w| (w.surface, w.reading)).collect()
    }

    #[test]
    fn clock_date_uses_month_and_irregular_day_readings() {
        let words = readings("11月24日");
        assert_eq!(words[0], ("11月".to_owned(), "じゅういちがつ".to_owned()));
        assert_eq!(words[1], ("24日".to_owned(), "にじゅうよっか".to_owned()));
    }

    #[test]
    fn irregular_days_and_full_width_digits() {
        assert_eq!(readings("1日")[0].1, "ついたち");
        assert_eq!(readings("２０日")[0].1, "はつか");
        assert_eq!(readings("4月")[0].1, "しがつ");
    }

    #[test]
    fn regular_counters_fix_only_the_counter_reading() {
        let words = readings("2026年");
        assert_eq!(words.last().unwrap(), &("年".to_owned(), "ねん".to_owned()));
    }

    #[test]
    fn out_of_range_numbers_are_left_alone() {
        assert_eq!(readings("32日").len(), 2);
    }
}
