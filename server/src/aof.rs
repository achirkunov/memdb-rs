use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};

use memdb_protocol::{Command, RespFrame};

use crate::store::Store;

pub struct AofWriter {
    file: File,
}

impl AofWriter {
    pub fn new(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(AofWriter { file })
    }

    pub fn is_write_command(frame: &RespFrame) -> bool {
        if let RespFrame::Arrays(items) = frame
            && let Some(RespFrame::BulkStrings(Some(name))) = items.first()
        {
            return matches!(name.to_ascii_uppercase().as_str(), "SET" | "DEL");
        }
        false
    }

    pub fn replay(path: &str, store: &mut Store) -> io::Result<usize> {
        let data = match fs::read(path) {
            Ok(data) => data,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e),
        };

        let mut remaining = &data[..];
        let mut count = 0;
        while let Ok((frame, rest)) = RespFrame::parse_bytes(remaining) {
            remaining = rest;
            match Command::from_frame(frame) {
                Ok(cmd) => {
                    store.execute(cmd);
                    count += 1;
                }
                Err(e) => {
                    eprintln!("AOF replay: skipping invalid command: {}", e);
                }
            }
        }
        Ok(count)
    }

    pub fn write(&mut self, frame: &RespFrame) -> io::Result<()> {
        let bytes = frame.marshal();
        self.file.write_all(&bytes)?;
        self.file.sync_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> String {
        let path = std::env::temp_dir().join(format!("memdb_test_{}", name));
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_aof_write_creates_file() {
        let path = temp_path("creates_file");
        let _ = fs::remove_file(&path);

        let mut aof = AofWriter::new(&path).unwrap();
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("SET".to_string())),
            RespFrame::BulkStrings(Some("key".to_string())),
            RespFrame::BulkStrings(Some("value".to_string())),
        ]);
        aof.write(&frame).unwrap();

        let contents = fs::read(&path).unwrap();
        assert_eq!(contents, frame.marshal());

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_aof_write_appends() {
        let path = temp_path("appends");
        let _ = fs::remove_file(&path);

        let mut aof = AofWriter::new(&path).unwrap();
        let frame1 = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("SET".to_string())),
            RespFrame::BulkStrings(Some("k1".to_string())),
            RespFrame::BulkStrings(Some("v1".to_string())),
        ]);
        let frame2 = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("SET".to_string())),
            RespFrame::BulkStrings(Some("k2".to_string())),
            RespFrame::BulkStrings(Some("v2".to_string())),
        ]);
        aof.write(&frame1).unwrap();
        aof.write(&frame2).unwrap();

        let contents = fs::read(&path).unwrap();
        let mut expected = frame1.marshal();
        expected.extend(frame2.marshal());
        assert_eq!(contents, expected);

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_replay_set() {
        let path = temp_path("replay_set");
        let _ = fs::remove_file(&path);

        // Write two SET commands to AOF file
        let mut aof = AofWriter::new(&path).unwrap();
        aof.write(&RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("SET".to_string())),
            RespFrame::BulkStrings(Some("k1".to_string())),
            RespFrame::BulkStrings(Some("v1".to_string())),
        ]))
        .unwrap();
        aof.write(&RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("SET".to_string())),
            RespFrame::BulkStrings(Some("k2".to_string())),
            RespFrame::BulkStrings(Some("v2".to_string())),
        ]))
        .unwrap();
        drop(aof);

        // Replay into fresh store
        let mut store = Store::new();
        let count = AofWriter::replay(&path, &mut store).unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            store.execute(memdb_protocol::Command::Get("k1".into())),
            RespFrame::BulkStrings(Some("v1".into()))
        );
        assert_eq!(
            store.execute(memdb_protocol::Command::Get("k2".into())),
            RespFrame::BulkStrings(Some("v2".into()))
        );

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_replay_del() {
        let path = temp_path("replay_del");
        let _ = fs::remove_file(&path);

        let mut aof = AofWriter::new(&path).unwrap();
        aof.write(&RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("SET".to_string())),
            RespFrame::BulkStrings(Some("k1".to_string())),
            RespFrame::BulkStrings(Some("v1".to_string())),
        ]))
        .unwrap();
        aof.write(&RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("DEL".to_string())),
            RespFrame::BulkStrings(Some("k1".to_string())),
        ]))
        .unwrap();
        drop(aof);

        let mut store = Store::new();
        let count = AofWriter::replay(&path, &mut store).unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            store.execute(memdb_protocol::Command::Get("k1".into())),
            RespFrame::BulkStrings(None)
        );

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_replay_empty_file() {
        let path = temp_path("replay_empty");
        let _ = fs::remove_file(&path);
        fs::write(&path, b"").unwrap();

        let mut store = Store::new();
        let count = AofWriter::replay(&path, &mut store).unwrap();
        assert_eq!(count, 0);

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_replay_missing_file() {
        let path = temp_path("replay_missing");
        let _ = fs::remove_file(&path);

        let mut store = Store::new();
        let count = AofWriter::replay(&path, &mut store).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_aof_write_del() {
        let path = temp_path("del");
        let _ = fs::remove_file(&path);

        let mut aof = AofWriter::new(&path).unwrap();
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("DEL".to_string())),
            RespFrame::BulkStrings(Some("key".to_string())),
        ]);
        aof.write(&frame).unwrap();

        let contents = fs::read(&path).unwrap();
        assert_eq!(contents, frame.marshal());

        fs::remove_file(&path).unwrap();
    }
}
