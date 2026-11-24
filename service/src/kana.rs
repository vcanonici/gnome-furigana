//! Character classification and katakana → hiragana conversion.

const KATAKANA_FIRST: u32 = 0x30A1;
const KATAKANA_LAST: u32 = 0x30F6;
const KATAKANA_TO_HIRAGANA: u32 = 0x60;

/// Returns true for CJK ideographs (BMP and supplementary planes) and marks that
/// behave like kanji inside words: 々 (時々), 〆 (〆切) and ヶ (一ヶ月).
#[must_use]
pub fn is_kanji(character: char) -> bool {
    matches!(character as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF | 0x2_0000..=0x3_134F)
        || matches!(character, '々' | '〆' | 'ヶ')
}

/// Returns true for hiragana, katakana and the prolonged sound mark ー.
#[must_use]
pub fn is_kana(character: char) -> bool {
    matches!(character as u32, 0x3041..=0x30FF)
}

/// Returns true when the text contains at least one kanji.
#[must_use]
pub fn has_kanji(text: &str) -> bool {
    text.chars().any(is_kanji)
}

/// Converts katakana to hiragana; every other character (including ー) is kept.
#[must_use]
pub fn to_hiragana(text: &str) -> String {
    text.chars()
        .map(|character| {
            let code = character as u32;
            if (KATAKANA_FIRST..=KATAKANA_LAST).contains(&code) {
                char::from_u32(code - KATAKANA_TO_HIRAGANA).unwrap_or(character)
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_kanji_including_supplementary_plane_and_iteration_mark() {
        assert!(is_kanji('𠮷') && is_kanji('々') && is_kanji('設'));
        assert!(!is_kanji('ー') && !is_kanji('あ') && !is_kanji('A'));
    }

    #[test]
    fn converts_katakana_keeping_prolonged_mark() {
        assert_eq!(to_hiragana("ユーザー"), "ゆーざー");
        assert_eq!(to_hiragana("セッテイ"), "せってい");
    }
}
