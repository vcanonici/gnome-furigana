//! Reading and writing GNU gettext `.mo` catalogs.
//!
//! Reads revision 0.x files of either byte order. System-dependent string tables
//! (minor revision 1) are ignored: those messages are simply not copied, and glibc
//! then takes them from the next language in `LANGUAGE` (the original `ja`).
//! Writes little-endian revision 0 without a hash table; glibc falls back to a
//! binary search, which requires originals sorted byte-wise.

const MAGIC_LITTLE: u32 = 0x9504_12de;
const MAGIC_BIG: u32 = 0xde12_0495;
const HEADER_LEN: usize = 28;

/// One catalog entry; `original` may contain `msgctxt\x04msgid` and plural `\0` parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Raw msgid (with context and plural forms as stored).
    pub original: Vec<u8>,
    /// Raw msgstr (plural forms separated by `\0`).
    pub translation: Vec<u8>,
}

/// Errors while decoding a catalog.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MoError {
    /// The file does not start with the gettext magic number.
    #[error("not a gettext catalog")]
    BadMagic,
    /// Major revision other than 0.
    #[error("unsupported .mo revision {0}")]
    UnsupportedRevision(u32),
    /// A table or string points outside the file.
    #[error("truncated or corrupt catalog")]
    Truncated,
}

struct Reader<'a> {
    bytes: &'a [u8],
    big_endian: bool,
}

impl Reader<'_> {
    fn word(&self, offset: usize) -> Result<u32, MoError> {
        let raw: [u8; 4] =
            self.bytes.get(offset..offset + 4).ok_or(MoError::Truncated)?.try_into().map_err(|_| MoError::Truncated)?;
        Ok(if self.big_endian { u32::from_be_bytes(raw) } else { u32::from_le_bytes(raw) })
    }

    fn string(&self, table: usize, index: usize) -> Result<Vec<u8>, MoError> {
        let entry = table + index * 8;
        let length = self.word(entry)? as usize;
        let offset = self.word(entry + 4)? as usize;
        Ok(self.bytes.get(offset..offset + length).ok_or(MoError::Truncated)?.to_vec())
    }
}

/// Decodes all regular messages of a catalog.
///
/// # Errors
/// Fails on a wrong magic number, a major revision other than 0, or corrupt tables.
pub fn parse(bytes: &[u8]) -> Result<Vec<Message>, MoError> {
    let magic = bytes.get(..4).map(|m| u32::from_le_bytes([m[0], m[1], m[2], m[3]])).ok_or(MoError::BadMagic)?;
    let big_endian = match magic {
        MAGIC_LITTLE => false,
        MAGIC_BIG => true,
        _ => return Err(MoError::BadMagic),
    };
    let reader = Reader { bytes, big_endian };
    let revision = reader.word(4)?;
    if revision >> 16 != 0 {
        return Err(MoError::UnsupportedRevision(revision >> 16));
    }
    let count = reader.word(8)? as usize;
    let (originals, translations) = (reader.word(12)? as usize, reader.word(16)? as usize);
    (0..count)
        .map(|index| {
            Ok(Message { original: reader.string(originals, index)?, translation: reader.string(translations, index)? })
        })
        .collect()
}

fn push_word(buffer: &mut Vec<u8>, value: usize) {
    // Catalog sizes are bounded far below 4 GiB; saturate instead of wrapping.
    buffer.extend_from_slice(&u32::try_from(value).unwrap_or(u32::MAX).to_le_bytes());
}

/// Encodes messages as a little-endian revision 0 catalog (sorted, no hash table).
#[must_use]
pub fn serialize(messages: &[Message]) -> Vec<u8> {
    let mut sorted: Vec<&Message> = messages.iter().collect();
    sorted.sort_by(|a, b| a.original.cmp(&b.original));
    let count = sorted.len();
    let originals_table = HEADER_LEN;
    let translations_table = originals_table + count * 8;
    let data_offset = translations_table + count * 8;
    let mut tables = Vec::with_capacity(count * 16);
    let mut data = Vec::new();
    for strings in
        [sorted.iter().map(|m| &m.original).collect::<Vec<_>>(), sorted.iter().map(|m| &m.translation).collect()]
    {
        for string in strings {
            push_word(&mut tables, string.len());
            push_word(&mut tables, data_offset + data.len());
            data.extend_from_slice(string);
            data.push(0);
        }
    }
    let mut file = Vec::with_capacity(data_offset + data.len());
    // magic, revision, count, originals, translations, hash size (none), hash offset.
    for value in [MAGIC_LITTLE as usize, 0, count, originals_table, translations_table, 0, data_offset] {
        push_word(&mut file, value);
    }
    file.extend(tables);
    file.extend(data);
    file
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(original: &str, translation: &str) -> Message {
        Message { original: original.as_bytes().to_vec(), translation: translation.as_bytes().to_vec() }
    }

    #[test]
    fn round_trips_context_plurals_and_header() {
        let messages = vec![
            message("", "Content-Type: text/plain; charset=UTF-8\n"),
            message("power\u{4}Off", "電源オフ"),
            message("%d file\u{0}%d files", "%d 個のファイル"),
        ];
        let parsed = parse(&serialize(&messages)).unwrap();
        let mut expected = messages.clone();
        expected.sort_by(|a, b| a.original.cmp(&b.original));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn reads_a_real_system_catalog() {
        let path = "/usr/share/locale-langpack/ja/LC_MESSAGES/gnome-control-center-2.0.mo";
        let Ok(bytes) = std::fs::read(path) else { return };
        let messages = parse(&bytes).unwrap();
        assert!(messages.iter().any(|m| m.translation == "電源管理".as_bytes()));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse(b"not a catalog at all, no"), Err(MoError::BadMagic));
    }
}
