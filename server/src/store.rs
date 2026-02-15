use std::collections::HashMap;

use memdb_protocol::{Command, RespFrame, Value};

pub struct Store {
    data: HashMap<String, Value>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn execute(&mut self, cmd: Command) -> RespFrame {
        match cmd {
            Command::Ping(None) => RespFrame::SimpleString("PONG".to_string()),
            Command::Ping(Some(msg)) => RespFrame::BulkStrings(Some(msg)),
            Command::Command => RespFrame::SimpleString("OK".to_string()),
            Command::Set(key, value) => {
                self.set(&key, value);
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

    fn set(&mut self, key: &str, value: Value) -> bool {
        self.data.insert(key.to_string(), value).is_none()
    }

    fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    fn del(&mut self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(store: &mut Store, key: &str, val: &str) {
        store.execute(Command::Set(key.into(), Value::String(val.into())));
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
}
