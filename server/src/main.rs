use std::collections::HashMap;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread;

use memdb_protocol::{Command, Response, Value};

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
                Response::OK
            },
            _ => todo!()
            // Command::Get(key) => {
            //     let value = self.get(&key);
            //     Response::Get(value.cloned()) // TODO: We will need to avoid using Clone since we will serialize the response anyway
            // },
            // Command::Del(key) => {
            //     let deleted = self.del(&key);
            //     Response::Del(deleted)
            // },
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

fn handle_client(stream: TcpStream) {
    // ...
}

// TODO: Replace with Box<dyn>
fn main() -> std::io::Result<()> {

    let mut port = "6379".to_string();
    let mut bind = "127.0.0.1".to_string();

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                i += 1;
                port = args[i].clone();
            }
            "--bind" | "-b" => {
                i += 1;
                bind = args[i].clone();
            }
            _ => eprintln!("unknown arg: {}", args[i]),
        }
        i += 1;
    }

    let mut store = Store::new();

    let addr = format!("{}:{}", bind, port);
    println!("listening on {}", addr);
    let listener = TcpListener::bind(&addr)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    handle_client(stream);
                });
            },
            Err(e) => eprintln!("connection failed: {}", e),
        }
    }

    Ok(())


}




