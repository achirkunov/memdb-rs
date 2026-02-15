use std::collections::HashMap;
use std::time::{Duration, Instant};

use memdb_protocol::{Command, RespFrame, Value};

pub struct Store {
    data: HashMap<String, Value>,
    expires: HashMap<String, Instant>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            expires: HashMap::new(),
        }
    }

    pub fn execute(&mut self, cmd: Command) -> RespFrame {
        match cmd {
            Command::Ping(None) => RespFrame::SimpleString("PONG".to_string()),
            Command::Ping(Some(msg)) => RespFrame::BulkStrings(Some(msg)),
            Command::Command => RespFrame::SimpleString("OK".to_string()),
            Command::Set(key, value, ex) => {
                self.set(&key, value, ex);
                RespFrame::SimpleString("OK".to_string())
            }
            Command::Get(key) => {
                match self.get(&key) {
                    Some(Value::String(s)) => RespFrame::BulkStrings(Some(s.clone())),
                    Some(_) => RespFrame::SimpleError(
                        "WRONGTYPE Operation against a key holding wrong kind of value".to_string(),
                    ),
                    None => RespFrame::BulkStrings(None), // $-1\r\n
                }
            }
            Command::Del(keys) => {
                let count = keys.into_iter().filter(|key| self.del(key)).count();
                RespFrame::Integer(count as i64)
            }
            Command::Echo(msg) => RespFrame::BulkStrings(Some(msg)),
        }
    }

    fn set(&mut self, key: &str, value: Value, ex: Option<u64>) -> bool {
        let is_new = self.data.insert(key.to_string(), value).is_none();
        match ex {
            Some(secs) => {
                self.expires
                    .insert(key.to_string(), Instant::now() + Duration::from_secs(secs));
            }
            None => {
                self.expires.remove(key);
            }
        }
        is_new
    }

    fn get(&mut self, key: &str) -> Option<&Value> {
        if self.is_expired(key) {
            self.data.remove(key);
            self.expires.remove(key);
            return None;
        }
        self.data.get(key)
    }

    fn del(&mut self, key: &str) -> bool {
        if self.is_expired(key) {
            self.data.remove(key);
            self.expires.remove(key);
            return false;
        }
        self.expires.remove(key);
        self.data.remove(key).is_some()
    }

    fn is_expired(&self, key: &str) -> bool {
        match self.expires.get(key) {
            Some(&deadline) => Instant::now() >= deadline,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(store: &mut Store, key: &str, val: &str) {
        store.execute(Command::Set(key.into(), Value::String(val.into()), None));
    }

    fn del(store: &mut Store, keys: Vec<&str>) -> RespFrame {
        store.execute(Command::Del(keys.into_iter().map(String::from).collect()))
    }

    #[test]
    fn del_existing_key() {
        let mut store = Store::new();
        set(&mut store, "k", "v");
        assert_eq!(del(&mut store, vec!["k"]), RespFrame::Integer(1));
    }

    #[test]
    fn del_non_existing_key() {
        let mut store = Store::new();
        assert_eq!(del(&mut store, vec!["k"]), RespFrame::Integer(0));
    }

    #[test]
    fn del_multiple_all_exist() {
        let mut store = Store::new();
        set(&mut store, "k1", "v");
        set(&mut store, "k2", "v");
        assert_eq!(del(&mut store, vec!["k1", "k2"]), RespFrame::Integer(2));
    }

    #[test]
    fn del_multiple_some_exist() {
        let mut store = Store::new();
        set(&mut store, "k1", "v");
        assert_eq!(
            del(&mut store, vec!["k1", "k2", "k3"]),
            RespFrame::Integer(1)
        );
    }

    #[test]
    fn del_same_key_twice() {
        let mut store = Store::new();
        set(&mut store, "k", "v");
        assert_eq!(del(&mut store, vec!["k", "k"]), RespFrame::Integer(1));
    }

    fn set_ex(store: &mut Store, key: &str, val: &str, secs: u64) {
        store.execute(Command::Set(
            key.into(),
            Value::String(val.into()),
            Some(secs),
        ));
    }

    #[test]
    fn get_before_expiry() {
        let mut store = Store::new();
        set_ex(&mut store, "k", "v", 1);
        assert_eq!(
            store.execute(Command::Get("k".into())),
            RespFrame::BulkStrings(Some("v".into()))
        );
    }

    #[test]
    fn get_after_expiry() {
        let mut store = Store::new();
        set_ex(&mut store, "k", "v", 1);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert_eq!(
            store.execute(Command::Get("k".into())),
            RespFrame::BulkStrings(None)
        );
    }

    #[test]
    fn set_without_ex_clears_expiry() {
        let mut store = Store::new();
        set_ex(&mut store, "k", "v1", 1);
        set(&mut store, "k", "v2");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert_eq!(
            store.execute(Command::Get("k".into())),
            RespFrame::BulkStrings(Some("v2".into()))
        );
    }

    #[test]
    fn del_expired_key_returns_zero() {
        let mut store = Store::new();
        set_ex(&mut store, "k", "v", 1);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert_eq!(del(&mut store, vec!["k"]), RespFrame::Integer(0));
    }

    #[test]
    fn set_overwrite_with_new_ex() {
        let mut store = Store::new();
        set_ex(&mut store, "k", "v1", 1);
        set_ex(&mut store, "k", "v2", 10);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert_eq!(
            store.execute(Command::Get("k".into())),
            RespFrame::BulkStrings(Some("v2".into()))
        );
    }

    #[test]
    fn del_removes_expiry() {
        let mut store = Store::new();
        set_ex(&mut store, "k", "v1", 10);
        del(&mut store, vec!["k"]);
        set(&mut store, "k", "v2");
        assert_eq!(
            store.execute(Command::Get("k".into())),
            RespFrame::BulkStrings(Some("v2".into()))
        );
    }
}
