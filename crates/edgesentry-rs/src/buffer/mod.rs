//! Offline buffer / store-and-forward for resilience (CLS-09).
//!
//! When an edge device loses connectivity it can push signed [`AuditRecord`]s into an
//! [`OfflineBuffer`].  When the link recovers the caller calls [`OfflineBuffer::flush`] which
//! replays the buffered records through an [`IngestService`] in the original order and reports
//! how many were accepted.
//!
//! # Pluggable storage
//! [`BufferStore`] is a synchronous trait.  The default implementation is
//! [`InMemoryBufferStore`] (volatile, useful for tests and embedded environments with no
//! persistent storage).  An optional SQLite-backed store is available behind the
//! `buffer-sqlite` feature flag ([`SqliteBufferStore`]).
//!
//! # Flush semantics
//! Records are replayed oldest-first.  A record that returns
//! [`IngestServiceError::Verify(IngestError::Duplicate)`] is treated as "already delivered"
//! and counted as accepted.  Any other error stops the flush at that point and the
//! remaining records stay in the buffer.

use thiserror::Error;

use crate::record::AuditRecord;
use crate::ingest::{IngestService, IngestServiceError, RawDataStore, AuditLedger, OperationLogStore};
use crate::ingest::IngestError;

/// A single entry in the offline buffer.
#[derive(Debug, Clone)]
pub struct BufferedEntry {
    pub record: AuditRecord,
    pub raw_payload: Vec<u8>,
}

/// Pluggable persistent / volatile storage for [`OfflineBuffer`].
pub trait BufferStore {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Append an entry to the back of the buffer.
    fn push(&mut self, entry: BufferedEntry) -> Result<(), Self::Error>;

    /// Return all entries in insertion order.
    fn entries(&self) -> Result<Vec<BufferedEntry>, Self::Error>;

    /// Remove the oldest `n` entries.  If `n >= len` the buffer is emptied.
    fn drop_oldest(&mut self, n: usize) -> Result<(), Self::Error>;

    /// Remove every entry.
    fn clear(&mut self) -> Result<(), Self::Error>;

    /// Return the current number of buffered entries.
    fn len(&self) -> Result<usize, Self::Error>;

    /// Return `true` when there are no buffered entries.
    fn is_empty(&self) -> Result<bool, Self::Error> {
        Ok(self.len()? == 0)
    }
}

/// Report returned by [`OfflineBuffer::flush`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlushReport {
    /// Number of records that were accepted (or already known to the service as duplicates).
    pub accepted: usize,
    /// Number of records that remain in the buffer after the flush.
    pub remaining: usize,
}

/// Error returned by [`OfflineBuffer::flush`].
#[derive(Debug, Error)]
pub enum FlushError<SE>
where
    SE: std::error::Error + Send + Sync + 'static,
{
    #[error("buffer store error: {0}")]
    Store(SE),
    #[error("ingest service error: {0}")]
    Ingest(IngestServiceError),
}

/// A store-and-forward buffer that accumulates signed [`AuditRecord`]s during connectivity loss
/// and replays them in order when the link recovers.
pub struct OfflineBuffer<S: BufferStore> {
    store: S,
}

impl<S: BufferStore> OfflineBuffer<S> {
    /// Create a new buffer backed by `store`.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Push a signed record into the buffer.
    pub fn push(&mut self, record: AuditRecord, raw_payload: Vec<u8>) -> Result<(), S::Error> {
        self.store.push(BufferedEntry { record, raw_payload })
    }

    /// Return the number of records currently buffered.
    pub fn len(&self) -> Result<usize, S::Error> {
        self.store.len()
    }

    /// Return `true` when the buffer is empty.
    pub fn is_empty(&self) -> Result<bool, S::Error> {
        self.store.is_empty()
    }

    /// Replay buffered records through `service` and return a [`FlushReport`].
    ///
    /// Records are submitted oldest-first.  A record that is accepted **or** already a
    /// duplicate is removed from the buffer.  The first non-duplicate error stops the
    /// replay; any remaining records are preserved for the next flush attempt.
    pub fn flush<R, L, O>(
        &mut self,
        service: &mut IngestService<R, L, O>,
    ) -> Result<FlushReport, FlushError<S::Error>>
    where
        R: RawDataStore,
        L: AuditLedger,
        O: OperationLogStore,
    {
        let entries = self.store.entries().map_err(FlushError::Store)?;
        let total = entries.len();
        let mut accepted = 0usize;

        for entry in &entries {
            match service.ingest(entry.record.clone(), &entry.raw_payload, None) {
                Ok(()) => {
                    accepted += 1;
                }
                Err(IngestServiceError::Verify(IngestError::Duplicate { .. })) => {
                    // Already delivered — count it and move on.
                    accepted += 1;
                }
                Err(other) => {
                    // Unrecoverable error for this record; stop here.
                    self.store.drop_oldest(accepted).map_err(FlushError::Store)?;
                    return Err(FlushError::Ingest(other));
                }
            }
        }

        self.store.drop_oldest(accepted).map_err(FlushError::Store)?;
        let remaining = total.saturating_sub(accepted);
        Ok(FlushReport { accepted, remaining })
    }
}

// ---------------------------------------------------------------------------
// InMemoryBufferStore
// ---------------------------------------------------------------------------

/// Volatile in-memory implementation of [`BufferStore`].  Entries are lost on drop.
#[derive(Default)]
pub struct InMemoryBufferStore {
    entries: Vec<BufferedEntry>,
}

impl InMemoryBufferStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Error)]
#[error("in-memory buffer store error: {message}")]
pub struct InMemoryBufferStoreError {
    message: String,
}

impl BufferStore for InMemoryBufferStore {
    type Error = InMemoryBufferStoreError;

    fn push(&mut self, entry: BufferedEntry) -> Result<(), Self::Error> {
        self.entries.push(entry);
        Ok(())
    }

    fn entries(&self) -> Result<Vec<BufferedEntry>, Self::Error> {
        Ok(self.entries.clone())
    }

    fn drop_oldest(&mut self, n: usize) -> Result<(), Self::Error> {
        let drain = n.min(self.entries.len());
        self.entries.drain(..drain);
        Ok(())
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        self.entries.clear();
        Ok(())
    }

    fn len(&self) -> Result<usize, Self::Error> {
        Ok(self.entries.len())
    }
}

// ---------------------------------------------------------------------------
// SqliteBufferStore (optional feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "buffer-sqlite")]
pub mod sqlite {
    use super::{BufferedEntry, BufferStore};
    use crate::record::AuditRecord;
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum SqliteBufferStoreError {
        #[error("sqlite error: {0}")]
        Sqlite(#[from] rusqlite::Error),
        #[error("deserialize error: {0}")]
        Deserialize(String),
    }

    /// SQLite-backed implementation of [`BufferStore`].
    ///
    /// Records are stored as JSON in a `buffer` table so they survive process restarts.
    pub struct SqliteBufferStore {
        conn: rusqlite::Connection,
    }

    impl SqliteBufferStore {
        /// Open (or create) the SQLite database at `path`.
        pub fn open(path: &str) -> Result<Self, SqliteBufferStoreError> {
            let conn = rusqlite::Connection::open(path)?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS buffer (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    record_json TEXT    NOT NULL,
                    payload_hex TEXT    NOT NULL
                );",
            )?;
            Ok(Self { conn })
        }

        /// Open an in-memory SQLite database (useful for tests).
        pub fn open_in_memory() -> Result<Self, SqliteBufferStoreError> {
            let conn = rusqlite::Connection::open_in_memory()?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS buffer (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    record_json TEXT    NOT NULL,
                    payload_hex TEXT    NOT NULL
                );",
            )?;
            Ok(Self { conn })
        }
    }

    impl BufferStore for SqliteBufferStore {
        type Error = SqliteBufferStoreError;

        fn push(&mut self, entry: BufferedEntry) -> Result<(), Self::Error> {
            let record_json = serde_json::to_string(&entry.record)
                .map_err(|e| SqliteBufferStoreError::Deserialize(e.to_string()))?;
            let payload_hex = hex::encode(&entry.raw_payload);
            self.conn.execute(
                "INSERT INTO buffer (record_json, payload_hex) VALUES (?1, ?2)",
                rusqlite::params![record_json, payload_hex],
            )?;
            Ok(())
        }

        fn entries(&self) -> Result<Vec<BufferedEntry>, Self::Error> {
            let mut stmt = self
                .conn
                .prepare("SELECT record_json, payload_hex FROM buffer ORDER BY id ASC")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;

            let mut entries = Vec::new();
            for row in rows {
                let (record_json, payload_hex) = row?;
                let record: AuditRecord = serde_json::from_str(&record_json)
                    .map_err(|e| SqliteBufferStoreError::Deserialize(e.to_string()))?;
                let raw_payload = hex::decode(&payload_hex)
                    .map_err(|e| SqliteBufferStoreError::Deserialize(e.to_string()))?;
                entries.push(BufferedEntry { record, raw_payload });
            }
            Ok(entries)
        }

        fn drop_oldest(&mut self, n: usize) -> Result<(), Self::Error> {
            if n == 0 {
                return Ok(());
            }
            self.conn.execute(
                "DELETE FROM buffer WHERE id IN (
                    SELECT id FROM buffer ORDER BY id ASC LIMIT ?1
                )",
                rusqlite::params![n as i64],
            )?;
            Ok(())
        }

        fn clear(&mut self) -> Result<(), Self::Error> {
            self.conn.execute("DELETE FROM buffer", [])?;
            Ok(())
        }

        fn len(&self) -> Result<usize, Self::Error> {
            let count: i64 =
                self.conn.query_row("SELECT COUNT(*) FROM buffer", [], |r| r.get(0))?;
            Ok(count as usize)
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build_lift_inspection_demo_records_with_payloads,
        ingest::{
            InMemoryAuditLedger, InMemoryOperationLog, InMemoryRawDataStore,
            IngestService, IntegrityPolicyGate,
        },
        parse_fixed_hex,
        record::AuditRecord,
    };
    use ed25519_dalek::SigningKey;

    const PRIV_HEX: &str = "0101010101010101010101010101010101010101010101010101010101010101";

    fn make_service() -> IngestService<InMemoryRawDataStore, InMemoryAuditLedger, InMemoryOperationLog> {
        let key_bytes = parse_fixed_hex::<32>(PRIV_HEX).unwrap();
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();

        let mut policy = IntegrityPolicyGate::new();
        policy.register_device("lift-01", verifying_key);
        IngestService::new(
            policy,
            InMemoryRawDataStore::default(),
            InMemoryAuditLedger::default(),
            InMemoryOperationLog::default(),
        )
    }

    fn demo_entries() -> Vec<(AuditRecord, Vec<u8>)> {
        build_lift_inspection_demo_records_with_payloads(
            "lift-01",
            PRIV_HEX,
            1_700_000_000_000,
            "s3://bucket/lift-01",
        )
        .unwrap()
    }

    #[test]
    fn push_increases_len() {
        let mut buf: OfflineBuffer<InMemoryBufferStore> =
            OfflineBuffer::new(InMemoryBufferStore::new());
        assert_eq!(buf.len().unwrap(), 0);

        let pairs = demo_entries();
        buf.push(pairs[0].0.clone(), pairs[0].1.clone()).unwrap();
        assert_eq!(buf.len().unwrap(), 1);
        buf.push(pairs[1].0.clone(), pairs[1].1.clone()).unwrap();
        assert_eq!(buf.len().unwrap(), 2);
    }

    #[test]
    fn flush_all_accepted_empties_buffer() {
        let mut buf: OfflineBuffer<InMemoryBufferStore> =
            OfflineBuffer::new(InMemoryBufferStore::new());
        let pairs = demo_entries();
        for (r, p) in &pairs {
            buf.push(r.clone(), p.clone()).unwrap();
        }

        let mut svc = make_service();
        let report = buf.flush(&mut svc).unwrap();

        assert_eq!(report.accepted, pairs.len());
        assert_eq!(report.remaining, 0);
        assert_eq!(buf.len().unwrap(), 0);
    }

    #[test]
    fn flush_duplicate_records_counted_as_accepted() {
        let mut buf: OfflineBuffer<InMemoryBufferStore> =
            OfflineBuffer::new(InMemoryBufferStore::new());
        let pairs = demo_entries();
        for (r, p) in &pairs {
            buf.push(r.clone(), p.clone()).unwrap();
        }

        let mut svc = make_service();
        // First flush delivers all records.
        let _ = buf.flush(&mut svc).unwrap();

        // Re-push the same records — they are now duplicates.
        for (r, p) in &pairs {
            buf.push(r.clone(), p.clone()).unwrap();
        }
        let report = buf.flush(&mut svc).unwrap();
        assert_eq!(report.accepted, pairs.len(), "duplicates should be counted as accepted");
        assert_eq!(report.remaining, 0);
    }

    #[test]
    fn flush_stops_on_unknown_device_error() {
        // Build a record for a device that is NOT registered in the service.
        let pairs = build_lift_inspection_demo_records_with_payloads(
            "unknown-device",
            PRIV_HEX,
            1_700_000_000_000,
            "s3://bucket/unknown",
        )
        .unwrap();

        let mut buf: OfflineBuffer<InMemoryBufferStore> =
            OfflineBuffer::new(InMemoryBufferStore::new());
        buf.push(pairs[0].0.clone(), pairs[0].1.clone()).unwrap();

        let mut svc = make_service(); // "unknown-device" is not registered
        let result = buf.flush(&mut svc);
        assert!(result.is_err(), "flush must fail when device is unknown");
        // The record should still be in the buffer.
        assert_eq!(buf.len().unwrap(), 1);
    }

    #[test]
    fn in_memory_store_drop_oldest_removes_entries() {
        let mut store = InMemoryBufferStore::new();
        let pairs = demo_entries();
        for (r, p) in &pairs {
            store.push(BufferedEntry { record: r.clone(), raw_payload: p.clone() }).unwrap();
        }
        assert_eq!(store.len().unwrap(), 3);
        store.drop_oldest(2).unwrap();
        assert_eq!(store.len().unwrap(), 1);
        let remaining = store.entries().unwrap();
        assert_eq!(remaining[0].record.sequence, pairs[2].0.sequence);
    }

    #[test]
    fn in_memory_store_clear_empties() {
        let mut store = InMemoryBufferStore::new();
        let pairs = demo_entries();
        for (r, p) in &pairs {
            store.push(BufferedEntry { record: r.clone(), raw_payload: p.clone() }).unwrap();
        }
        store.clear().unwrap();
        assert_eq!(store.len().unwrap(), 0);
    }

    #[cfg(feature = "buffer-sqlite")]
    mod sqlite_tests {
        use super::*;
        use crate::buffer::sqlite::SqliteBufferStore;

        #[test]
        fn sqlite_store_roundtrip() {
            let mut store = SqliteBufferStore::open_in_memory().unwrap();
            let pairs = demo_entries();
            for (r, p) in &pairs {
                store.push(BufferedEntry { record: r.clone(), raw_payload: p.clone() }).unwrap();
            }
            assert_eq!(store.len().unwrap(), 3);
            let entries = store.entries().unwrap();
            assert_eq!(entries[0].record.sequence, 1);
            assert_eq!(entries[2].record.sequence, 3);
        }

        #[test]
        fn sqlite_flush_works() {
            let store = SqliteBufferStore::open_in_memory().unwrap();
            let mut buf = OfflineBuffer::new(store);
            let pairs = demo_entries();
            for (r, p) in &pairs {
                buf.push(r.clone(), p.clone()).unwrap();
            }
            let mut svc = make_service();
            let report = buf.flush(&mut svc).unwrap();
            assert_eq!(report.accepted, 3);
            assert_eq!(report.remaining, 0);
        }
    }
}
