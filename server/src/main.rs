mod store;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use memdb_protocol::{Command, RespFrame};
use store::Store;

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

        // Drain all complete frames from the buffer before blocking on next read
        loop {
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
                            let bytes = RespFrame::SimpleError(e.to_string()).marshal();
                            stream.write_all(&bytes).unwrap();
                        }
                    }
                },
                Err(_) => break, // incomplete data, need more bytes
            }
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
