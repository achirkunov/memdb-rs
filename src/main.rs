use std::collections::HashMap;

enum Value {
    String(String),
    Integer(i64),
    List(Vec<Value>),
}

type Store = HashMap<String, Value>;

fn main() {

    let mut store = Store::new();
    store.insert("key".to_string(), Value::String("value".to_string()) );


    let v = store.get("key").unwrap();

    if let Value::String(s) = v {
        println!("v: {}", s);
    }

}
