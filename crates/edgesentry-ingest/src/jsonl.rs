use serde::{de::DeserializeOwned, Serialize};
use std::io::{BufRead, BufReader, Read, Write};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonlHeader {
    pub eds_schema: String,
    pub version: String,
}

pub struct JsonlReader<R: Read> {
    reader: BufReader<R>,
    pub header: JsonlHeader,
}

impl<R: Read> JsonlReader<R> {
    pub fn open(r: R) -> Result<Self, String> {
        let mut reader = BufReader::new(r);
        let mut first_line = String::new();
        reader.read_line(&mut first_line).map_err(|e| e.to_string())?;
        let header: JsonlHeader = serde_json::from_str(first_line.trim())
            .map_err(|e| format!("invalid JSONL header: {e}"))?;
        Ok(Self { reader, header })
    }

    pub fn records<T: DeserializeOwned>(&mut self) -> impl Iterator<Item = Result<T, String>> + '_ {
        self.reader.by_ref().lines().filter_map(|line| {
            let line = line.ok()?;
            let line = line.trim().to_string();
            if line.is_empty() { return None; }
            Some(serde_json::from_str(&line).map_err(|e| e.to_string()))
        })
    }
}

pub struct JsonlWriter<W: Write> {
    writer: W,
}

impl<W: Write> JsonlWriter<W> {
    pub fn new(mut w: W, schema: &str, version: &str) -> Result<Self, String> {
        let header = JsonlHeader {
            eds_schema: schema.to_string(),
            version: version.to_string(),
        };
        let line = serde_json::to_string(&header).map_err(|e| e.to_string())?;
        writeln!(w, "{line}").map_err(|e| e.to_string())?;
        Ok(Self { writer: w })
    }

    pub fn write_record<T: Serialize>(&mut self, record: &T) -> Result<(), String> {
        let line = serde_json::to_string(record).map_err(|e| e.to_string())?;
        writeln!(self.writer, "{line}").map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct TestRecord {
        id: u32,
        value: String,
    }

    #[test]
    fn roundtrip_single_record() {
        let mut buf = Vec::new();
        {
            let mut writer = JsonlWriter::new(&mut buf, "test.schema", "1.0").unwrap();
            writer.write_record(&TestRecord { id: 1, value: "hello".to_string() }).unwrap();
        }
        let cursor = Cursor::new(buf);
        let mut reader = JsonlReader::open(cursor).unwrap();
        assert_eq!(reader.header.eds_schema, "test.schema");
        assert_eq!(reader.header.version, "1.0");
        let records: Vec<TestRecord> = reader.records().map(|r| r.unwrap()).collect();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0], TestRecord { id: 1, value: "hello".to_string() });
    }

    #[test]
    fn roundtrip_multiple_records() {
        let mut buf = Vec::new();
        {
            let mut writer = JsonlWriter::new(&mut buf, "multi.schema", "2.0").unwrap();
            for i in 0..5u32 {
                writer.write_record(&TestRecord { id: i, value: format!("item-{i}") }).unwrap();
            }
        }
        let cursor = Cursor::new(buf);
        let mut reader = JsonlReader::open(cursor).unwrap();
        let records: Vec<TestRecord> = reader.records().map(|r| r.unwrap()).collect();
        assert_eq!(records.len(), 5);
        assert_eq!(records[3].id, 3);
        assert_eq!(records[3].value, "item-3");
    }

    #[test]
    fn header_schema_and_version_preserved() {
        let mut buf = Vec::new();
        JsonlWriter::new(&mut buf, "eds.entity-frame", "0.1").unwrap();
        let cursor = Cursor::new(buf);
        let reader = JsonlReader::open(cursor).unwrap();
        assert_eq!(reader.header.eds_schema, "eds.entity-frame");
        assert_eq!(reader.header.version, "0.1");
    }

    #[test]
    fn invalid_header_returns_error() {
        let data = b"not a json header\n{\"id\":1}\n";
        let cursor = Cursor::new(data.as_slice());
        assert!(JsonlReader::open(cursor).is_err());
    }

    #[test]
    fn empty_lines_are_skipped() {
        let mut buf = Vec::new();
        {
            let mut writer = JsonlWriter::new(&mut buf, "s", "1").unwrap();
            writer.write_record(&TestRecord { id: 1, value: "a".into() }).unwrap();
        }
        // Inject empty lines into the buffer after the header
        let mut text = String::from_utf8(buf).unwrap();
        text.push('\n');
        text.push('\n');
        let cursor = Cursor::new(text.into_bytes());
        let mut reader = JsonlReader::open(cursor).unwrap();
        let records: Vec<TestRecord> = reader.records().map(|r| r.unwrap()).collect();
        assert_eq!(records.len(), 1);
    }
}
