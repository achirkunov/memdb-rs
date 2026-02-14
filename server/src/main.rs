use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::sync::{Arc, Mutex};

use memdb_protocol::{Command, RespFrame, Value};

struct Store {
    data: HashMap<String,Value>,
}

impl Store {

    fn new() -> Self {
        Self {
            data: HashMap::new()
        }
    }

    fn execute(&mut self, cmd: Command) -> RespFrame {
        match cmd {
            Command::Ping => RespFrame::SimpleString("PONG".to_string()),
            Command::Command => RespFrame::SimpleString("OK".to_string()),
            Command::Set(key, value) => {
                self.set(&key, value);
                RespFrame::SimpleString("OK".to_string())
            },
            Command::Get(key) => {
                match self.get(&key) {
                    Some(Value::String(s)) => RespFrame::BulkStrings(Some(s.clone())),
                    Some(_) => RespFrame::SimpleError("WRONGTYPE Operation against a key holding wrong kind of value".to_string()),
                    None => RespFrame::BulkStrings(None), // $-1\r\n
                }
            },
            Command::Del(keys) => {
                let count = keys.into_iter().filter(|key| self.del(key)).count();
                RespFrame::Integer(count as i64)
            },
            _ => todo!()
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

fn handle_client(mut stream: TcpStream, store: Arc<Mutex<Store>>) {
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
            Ok((frame, remaining)) => {
                data = remaining.to_vec();
                match Command::from_frame(frame) {
                    Ok(cmd) => {
                        println!("Received cmd: {:?}", cmd);
                        let response = {
                            let mut store = store.lock().unwrap();
                            store.execute(cmd)
                        }; // lock released here
                        let bytes = response.marshal();
                        stream.write_all(&bytes).unwrap();
                    },
                    Err(e) => {
                        //let msg = format!("ERR: {:?}", e);
                        let bytes = RespFrame::SimpleError(e.to_string()).marshal();
                        stream.write_all(&bytes).unwrap();
                    }
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

    // We have one Store but multiple threads (one per client)
    // Arc: shared ownership across threads (ref-counted pointer)
    // Mutex: exclusive access for mutation
    let store = Arc::new(Mutex::new(Store::new()));


    let addr = format!("{}:{}", bind, port);
    println!("listening on {}", addr);
    let listener = TcpListener::bind(&addr)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let store = Arc::clone(&store); // increment ref count
                thread::spawn(move || {
                    handle_client(stream, store);
                });
            },
            Err(e) => eprintln!("connection failed: {}", e),
        }
    }

    Ok(())


}




