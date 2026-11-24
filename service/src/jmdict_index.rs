//! Compact SQLite index of the EDRDG `JMdict_e` dictionary (CC-BY-SA 4.0).
//!
//! The index is built once at install time and queried read-only, so the service
//! keeps the ~200k entries on disk instead of in memory.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Mutex;

use flate2::read::GzDecoder;
use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::glossary::{Entry, Source};

const MAX_SENSES: usize = 3;
const SCHEMA: &str = "CREATE TABLE entries (term TEXT NOT NULL, reading TEXT NOT NULL, glosses TEXT NOT NULL,
                      pos TEXT, priority INTEGER NOT NULL);
                      CREATE INDEX entries_term ON entries(term);";

/// Errors while building the index.
#[derive(Debug, thiserror::Error)]
pub enum JmdictBuildError {
    /// Reading the source or replacing the target failed.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// The XML is malformed.
    #[error("xml: {0}")]
    Xml(#[from] quick_xml::Error),
    /// Writing the SQLite index failed.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Serialising glosses failed.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// One row of the `entries` table.
struct IndexRow {
    term: String,
    reading: String,
    glosses: Vec<String>,
    part_of_speech: Option<String>,
    priority: u32,
}

/// One `<entry>` being accumulated while streaming the XML.
#[derive(Default)]
struct EntryDraft {
    kanji: Vec<(String, u32)>,
    readings: Vec<String>,
    senses: Vec<Vec<String>>,
    first_pos: Option<String>,
}

impl EntryDraft {
    /// Index rows; kana-only entries are keyed by their readings.
    fn rows(&self) -> Vec<IndexRow> {
        let glosses: Vec<String> =
            self.senses.iter().take(MAX_SENSES).map(|s| s.join("; ")).filter(|g| !g.is_empty()).collect();
        let Some(first_reading) = self.readings.first() else { return Vec::new() };
        if glosses.is_empty() {
            return Vec::new();
        }
        let keys: Vec<(String, u32)> = if self.kanji.is_empty() {
            self.readings.iter().map(|r| (r.clone(), 0)).collect()
        } else {
            self.kanji.clone()
        };
        keys.into_iter()
            .map(|(term, priority)| IndexRow {
                term,
                reading: first_reading.clone(),
                glosses: glosses.clone(),
                part_of_speech: self.first_pos.clone(),
                priority,
            })
            .collect()
    }

    fn push_text(&mut self, element: &str, text: &str) {
        match element {
            "keb" => self.kanji.push((text.to_owned(), 0)),
            "ke_pri" => {
                if let Some(last) = self.kanji.last_mut() {
                    last.1 += 1;
                }
            }
            "reb" => self.readings.push(text.to_owned()),
            "gloss" => self.senses.last_mut().into_iter().for_each(|sense| sense.push(text.to_owned())),
            "pos" if self.senses.len() == 1 && self.first_pos.is_none() => self.first_pos = Some(text.to_owned()),
            _ => {}
        }
    }
}

/// Streams `JMdict_e` (gzip or plain XML) into `target`, replacing it atomically.
/// Returns the number of indexed rows.
///
/// # Errors
/// Fails on IO, XML or `SQLite` errors; the previous index is kept in that case.
pub fn build_index(source: &Path, target: &Path) -> Result<usize, JmdictBuildError> {
    let temporary = target.with_extension("tmp");
    let _ = std::fs::remove_file(&temporary);
    let mut database = Connection::open(&temporary)?;
    database.execute_batch(SCHEMA)?;
    let transaction = database.transaction()?;
    let count = stream_entries(source, |draft| insert_rows(&transaction, draft))?;
    transaction.commit()?;
    drop(database);
    std::fs::rename(&temporary, target)?;
    Ok(count)
}

fn insert_rows(transaction: &rusqlite::Transaction, draft: &EntryDraft) -> Result<usize, JmdictBuildError> {
    let rows = draft.rows();
    for row in &rows {
        transaction.execute(
            "INSERT INTO entries VALUES (?1, ?2, ?3, ?4, ?5)",
            params![row.term, row.reading, serde_json::to_string(&row.glosses)?, row.part_of_speech, row.priority],
        )?;
    }
    Ok(rows.len())
}

fn open_source(source: &Path) -> Result<Box<dyn std::io::BufRead>, std::io::Error> {
    let file = File::open(source)?;
    if source.extension().is_some_and(|extension| extension == "gz") {
        Ok(Box::new(BufReader::new(GzDecoder::new(file))))
    } else {
        Ok(Box::new(BufReader::new(file)))
    }
}

/// Calls `on_entry` for every `<entry>`. Text of an element is accumulated until its
/// end tag because entity references split it into several events. DTD references
/// such as `&n;` are kept as their short code (`n`), which `JMdict` documents as the tag.
fn stream_entries(
    source: &Path,
    mut on_entry: impl FnMut(&EntryDraft) -> Result<usize, JmdictBuildError>,
) -> Result<usize, JmdictBuildError> {
    let mut reader = Reader::from_reader(open_source(source)?);
    let (mut buffer, mut text, mut draft, mut count) = (Vec::new(), String::new(), EntryDraft::default(), 0);
    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(start) => {
                text.clear();
                if start.name().as_ref() == "sense" {
                    draft.senses.push(Vec::new());
                }
            }
            Event::Text(content) => text.push_str(&content.xml_content(XmlVersion::Implicit1_0)),
            Event::GeneralRef(reference) => {
                text.push_str(&resolve_reference(&reference));
            }
            Event::End(end) if end.name().as_ref() == "entry" => count += on_entry(&std::mem::take(&mut draft))?,
            Event::End(end) => draft.push_text(end.name().as_ref(), &std::mem::take(&mut text)),
            Event::Eof => return Ok(count),
            _ => {}
        }
        buffer.clear();
    }
}

fn resolve_reference(name: &str) -> String {
    quick_xml::escape::resolve_predefined_entity(name).map_or_else(|| name.to_owned(), ToOwned::to_owned)
}

/// Read-only handle to a built index.
pub struct JmdictIndex {
    // rusqlite connections are not `Sync`; the D-Bus object must be.
    connection: Mutex<Connection>,
}

impl JmdictIndex {
    /// Opens the index read-only; returns `None` when it does not exist or is unreadable.
    #[must_use]
    pub fn open(path: &Path) -> Option<Self> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
        Some(Self { connection: Mutex::new(connection) })
    }

    /// Looks a term up, preferring the given reading and then the most frequent entry.
    #[must_use]
    pub fn lookup(&self, term: &str, reading: Option<&str>) -> Option<Entry> {
        let connection = self.connection.lock().ok()?;
        let row: Option<(String, String, Option<String>)> = connection
            .query_row(
                "SELECT reading, glosses, pos FROM entries WHERE term = ?1 ORDER BY (reading = ?2) DESC, priority DESC LIMIT 1",
                params![term, reading.unwrap_or_default()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .ok()
            .flatten();
        let (found_reading, glosses, part_of_speech) = row?;
        Some(Entry {
            term: term.to_owned(),
            reading: Some(found_reading),
            glosses: serde_json::from_str(&glosses).ok()?,
            part_of_speech,
            source: Source::Jmdict,
            language: "en",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE JMdict [
<!ENTITY n "noun (common) (futsuumeishi)">
]>
<JMdict>
<entry><ent_seq>1</ent_seq>
<k_ele><keb>睡眠</keb><ke_pri>news1</ke_pri></k_ele>
<r_ele><reb>すいみん</reb></r_ele>
<sense><pos>&n;</pos><gloss>sleep</gloss><gloss>slumber &amp; rest</gloss></sense>
</entry>
</JMdict>"#;

    #[test]
    fn builds_and_queries_index() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("JMdict_e.xml");
        std::fs::write(&source, SAMPLE).unwrap();
        let target = directory.path().join("jmdict.sqlite");
        assert_eq!(build_index(&source, &target).unwrap(), 1);
        let entry = JmdictIndex::open(&target).unwrap().lookup("睡眠", Some("すいみん")).unwrap();
        assert_eq!(entry.glosses, vec!["sleep; slumber & rest"]);
        assert_eq!((entry.part_of_speech.as_deref(), entry.language), (Some("n"), "en"));
    }
}
