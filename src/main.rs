use std::collections::HashMap;

#[derive(Clone)] // TODO: to remove
enum Value {
    String(String),
    Integer(i64),
    List(Vec<Value>),
}

//type Store = HashMap<String, Value>;

struct Store {
    data: HashMap<String,Value>,
}

impl Store {

    fn new() -> Self {
        Self {
            data: HashMap::new()
        }
    }

    fn execute(&mut self, cmd: Command) -> Response {
        match cmd {
            Command::Ping => Response::Pong,
            Command::Set(key, value) => {
                let was_new = self.set(&key, value);
                Response::Set(was_new)
            },
            Command::Get(key) => {
                let value = self.get(&key);
                Response::Get(value.cloned()) // TODO: We will need to avoid using Clone since we will serialize the response anyway
            },
            Command::Del(key) => {
                let deleted = self.del(&key);
                Response::Del(deleted)
            },
        }
    }

    fn set(&mut self, key: &str, value: Value) -> bool {
        let was_new = self.data.insert(key.to_string(), value).is_none();
        was_new
    }

    fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    fn del(&mut self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }
}

enum Command {
    Ping,
    Set(String, Value),
    Get(String),
    Del(String),
}

enum Response {
    Pong,
    Set(bool),
    Get(Option<Value>),
    Del(bool),
    Error(String),
}

fn main() {

    let mut store = Store::new();

    let was_new = store.set("user:1:name", Value::String("Alice".to_string()));
    println!("was new: {}", was_new);

    let v = store.get("user:1:name").unwrap();

    if let Value::String(s) = v {
        println!("v: {}", s);
    }

    let deleted = store.del("user:1:name");
    println!("deleted: {}", deleted);

}
