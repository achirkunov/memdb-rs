#[derive(Clone)] // TODO: to remove
pub enum Value {
    String(String),
    Integer(i64),
    List(Vec<Value>),
}

pub enum Command {
    Ping,
    Set(String, Value),
    Get(String),
    Del(String),
}

pub enum Response {
    Pong,
    Set(bool),
    Get(Option<Value>),
    Del(bool),
    Error(String),
}