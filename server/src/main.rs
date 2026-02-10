use std::collections::HashMap;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread;

use memdb_protocol::{Command, RespFrame, Response, Value};

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

fn handle_client(mut stream: TcpStream) {
    // ...
    let mut buf = [0u8; 1024];
    let mut data = Vec::new();

    loop {
        // 1. Read chunk from stream
        let n = match stream.read(&mut buf) {
            Ok(0) => return, // client disconnected
            Ok(n) => n,
            Err(e) => { eprintln!("read error: {}", e); return; }
        };
        data.extend_from_slice(&buf[..n]); // why do we need this?
        // See it as a string (RESP is mostly ASCII, so this is nice for debugging)
        println!("received: {:?}", String::from_utf8_lossy(&data));

        // TODO: Try parsing frame from accumulated data (loop - multiple frames possible in one read)
        match RespFrame::parse_bytes(&data) {
            Ok((frame,_)) => {
                match Command::from_frame(frame) {
                    Ok(cmd) => {
                        println!("Received cmd: {:?}", cmd);

                        // TODO: redis-cli sends COMMAND DOCS on startup to discover which commands the server supports
                    },
                    Err(e) => eprintln!("command error: {:?}", e)
                }
            },
            Err(_) => { eprintln!("Incomplete data"); break; }, // incomplete data
        }
    }

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




