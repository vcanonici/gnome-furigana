//! Adds inline readings to one translated message without touching anything an
//! application parses: markup, entities, printf/brace placeholders and mnemonics are
//! copied verbatim, and the result is rejected unless stripping the readings gives
//! back exactly the original text.

use crate::analysis::{Analyzer, MAX_TEXT_CHARS};
use crate::inline_readings::strip_inline_readings;
use crate::kana::has_kanji;

/// strftime directives that only make sense in date/time formats. A msgid using them
/// is a format string apps may also parse, so it is never annotated. `%s`/`%d`/`%x`
/// are excluded because printf uses them too.
const DATE_DIRECTIVES: &str = "aAbBcCeFgGhHIjklmMpPrRSTuUVwWyYzZ";

/// A piece of a message: text to annotate, or something copied verbatim.
#[derive(Debug, PartialEq, Eq)]
enum Piece<'a> {
    Text(&'a str),
    Protected(&'a str),
}

fn protected_length(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    match bytes.first()? {
        b'<' if bytes.get(1).is_some_and(|b| b.is_ascii_alphabetic() || b"/!?".contains(b)) => {
            rest.find('>').map(|end| end + 1)
        }
        b'&' => {
            let end = rest.find(';')?;
            rest[1..end].chars().all(|c| c.is_ascii_alphanumeric() || c == '#').then_some(end + 1)
        }
        b'%' => Some(printf_length(bytes)),
        b'{' | b'$' => brace_length(rest),
        _ => None,
    }
}

/// Length of a printf conversion such as `%s`, `%1$s`, `%-5.2f`, `%'d`, `%lld`, `%%`.
fn printf_length(bytes: &[u8]) -> usize {
    let mut index = 1;
    let skip = |index: &mut usize, accept: &dyn Fn(u8) -> bool| {
        while bytes.get(*index).copied().is_some_and(accept) {
            *index += 1;
        }
    };
    skip(&mut index, &|b: u8| b.is_ascii_digit() || b == b'$');
    skip(&mut index, &|b: u8| b"-+ #0'I".contains(&b));
    skip(&mut index, &|b: u8| b.is_ascii_digit() || b == b'*' || b == b'.');
    skip(&mut index, &|b: u8| b"hlLqjzt".contains(&b));
    // A literal percent before non-ASCII text (`50%の`) protects only the `%` itself;
    // never end inside a multi-byte character.
    match bytes.get(index) {
        Some(conversion) if conversion.is_ascii_alphabetic() || *conversion == b'%' => index + 1,
        _ => 1,
    }
}

/// Length of `{name}`, `{0}`, `{}` or `${name}` placeholders.
fn brace_length(rest: &str) -> Option<usize> {
    let open = usize::from(rest.starts_with('$'));
    if !rest[open..].starts_with('{') {
        return None;
    }
    let end = rest.find('}')?;
    rest[open + 1..end].chars().all(|c| c.is_ascii_alphanumeric() || "_.:".contains(c)).then_some(end + 1)
}

fn split_pieces(message: &str) -> Vec<Piece<'_>> {
    let mut pieces = Vec::new();
    let (mut text_start, mut index) = (0, 0);
    while index < message.len() {
        if let Some(length) = protected_length(&message[index..]) {
            if text_start < index {
                pieces.push(Piece::Text(&message[text_start..index]));
            }
            pieces.push(Piece::Protected(&message[index..index + length]));
            index += length;
            text_start = index;
        } else {
            index += message[index..].chars().next().map_or(1, char::len_utf8);
        }
    }
    if text_start < message.len() {
        pieces.push(Piece::Text(&message[text_start..]));
    }
    pieces
}

/// True when a msgid looks like a strftime format (`%Y年%m月%d日`, `%H:%M`).
#[must_use]
pub fn is_date_format(msgid: &str) -> bool {
    msgid.as_bytes().windows(2).any(|pair| pair[0] == b'%' && DATE_DIRECTIVES.as_bytes().contains(&pair[1]))
        || msgid.contains("%-")
}

/// Annotates one text run line by line (the analyzer accepts at most 512 chars).
fn annotate_text(analyzer: &mut Analyzer, text: &str) -> String {
    text.split_inclusive('\n')
        .map(|line| {
            if !has_kanji(line) || line.chars().count() > MAX_TEXT_CHARS {
                return line.to_owned();
            }
            analyzer.analyze(line).map_or_else(|_| line.to_owned(), |analysis| analysis.display_inline.clone())
        })
        .collect()
}

/// Returns the annotated message, or `None` when nothing changed or a safety check failed.
pub fn annotate_message(analyzer: &mut Analyzer, message: &str) -> Option<String> {
    if !has_kanji(message) {
        return None;
    }
    let annotated: String = split_pieces(message)
        .into_iter()
        .map(|piece| match piece {
            Piece::Text(text) => annotate_text(analyzer, text),
            Piece::Protected(text) => text.to_owned(),
        })
        .collect();
    let is_lossless = strip_inline_readings(&annotated).plain == strip_inline_readings(message).plain;
    (annotated != message && is_lossless).then_some(annotated)
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

    #[test]
    fn annotates_settings_strings_and_keeps_mnemonics() {
        let mut analyzer = analyzer();
        assert_eq!(annotate_message(&mut analyzer, "電源管理").unwrap(), "電源(でんげん)管理(かんり)");
        assert_eq!(
            annotate_message(&mut analyzer, "固定ワークスペース(_F)").unwrap(),
            "固定(こてい)ワークスペース(_F)"
        );
    }

    #[test]
    fn placeholders_markup_and_entities_are_copied_verbatim() {
        let mut analyzer = analyzer();
        let annotated = annotate_message(&mut analyzer, "<b>%1$s</b> を削除 &amp; {name} の設定 %d 個").unwrap();
        for protected in ["<b>", "%1$s", "</b>", "&amp;", "{name}", "%d"] {
            assert!(annotated.contains(protected), "{protected} missing in {annotated}");
        }
        assert!(annotated.contains("削除(さくじょ)") && annotated.contains("設定(せってい)"));
    }

    #[test]
    fn strings_without_kanji_or_already_annotated_are_left_alone() {
        let mut analyzer = analyzer();
        assert_eq!(annotate_message(&mut analyzer, "ワークスペース"), None);
        assert_eq!(annotate_message(&mut analyzer, "設定(せってい)"), None);
    }

    #[test]
    fn date_formats_are_detected() {
        assert!(is_date_format("%Y年%m月%d日"));
        assert!(is_date_format("%-d %B"));
        assert!(!is_date_format("%s を削除しますか？"));
        assert!(!is_date_format("%d 個のファイル"));
    }

    #[test]
    fn pieces_split_around_protected_tokens() {
        assert_eq!(split_pieces("a%sb"), vec![Piece::Text("a"), Piece::Protected("%s"), Piece::Text("b")]);
        assert_eq!(split_pieces("x < y"), vec![Piece::Text("x < y")]);
        assert_eq!(split_pieces("50%の容量"), vec![Piece::Text("50"), Piece::Protected("%"), Piece::Text("の容量")]);
    }
}
