mod aof;
mod store;

use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use aof::AofWriter;
use memdb_protocol::{Command, RespFrame};
use store::Store;

async fn handle_client(
    mut stream: TcpStream,
    store: Arc<Mutex<Store>>,
    aof: Option<Arc<Mutex<AofWriter>>>,
) {
    // ...
    let mut buf = [0u8; 1024];
    let mut data = Vec::new();

    loop {
        // 1. Read chunk from stream
        let n = match stream.read(&mut buf).await {
            Ok(0) => return, // client disconnected
            Ok(n) => n,
            Err(e) => {
                eprintln!("read error: {}", e);
                return;
            }
        };
        data.extend_from_slice(&buf[..n]); // why do we need this?
        // See it as a string (RESP is mostly ASCII, so this is nice for debugging)
        println!("received: {:?}", String::from_utf8_lossy(&data));

        // Drain all complete frames from the buffer before blocking on next read
        while let Ok((frame, remaining)) = RespFrame::parse_bytes(&data) {
            data = remaining.to_vec();

            // Append write commands to AOF before executing
            if let Some(ref aof) = aof
                && AofWriter::is_write_command(&frame)
                && let Err(e) = aof.lock().unwrap().write(&frame)
            {
                eprintln!("AOF write error: {}", e);
                continue;
            }

            match Command::from_frame(frame) {
                Ok(cmd) => {
                    println!("Received cmd: {:?}", cmd);
                    let response = {
                        let mut store = store.lock().unwrap();
                        store.execute(cmd)
                    }; // lock released here
                    let bytes = response.marshal();
                    stream.write_all(&bytes).await.unwrap();
                }
                Err(e) => {
                    let bytes = RespFrame::SimpleError(e.to_string()).marshal();
                    stream.write_all(&bytes).await.unwrap();
                }
            }
        }
    }
}

// TODO: Replace with Box<dyn>
// Single-threaded event loop like Redis: no threads, no locks.
// All client tasks run cooperatively on one OS thread via spawn_local.
#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let mut port = "6379".to_string();
    let mut bind = "127.0.0.1".to_string();
    let mut appendonly = false;
    let mut aof_file = "appendonly.aof".to_string();

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
            "--appendonly" => {
                appendonly = true;
            }
            "--aof-file" => {
                i += 1;
                aof_file = args[i].clone();
            }
            _ => eprintln!("unknown arg: {}", args[i]),
        }
        i += 1;
    }

    let mut store = Store::new();

    // Replay AOF to rebuild dataset before accepting connections
    let aof: Option<Arc<Mutex<AofWriter>>> = if appendonly {
        let count = AofWriter::replay(&aof_file, &mut store)?;
        println!(
            "AOF enabled, file: {}, replayed {} commands",
            aof_file, count
        );
        Some(Arc::new(Mutex::new(AofWriter::new(&aof_file)?)))
    } else {
        None
    };

    // Arc: shared ownership across threads (ref-counted pointer)
    // Mutex: exclusive access for mutation
    let store = Arc::new(Mutex::new(store));

    // Active expiration: background thread samples expired keys periodically
    // let expiry_store = Arc::clone(&store);
    // thread::spawn(move || {
    //     loop {
    //         thread::sleep(Duration::from_millis(100));
    //         let start = Instant::now();
    //         loop {
    //             let expired = expiry_store.lock().unwrap().evict_expired_sample(20);
    //             if expired * 4 <= 20 {
    //                 break;
    //             }
    //             if start.elapsed() > Duration::from_millis(25) {
    //                 break;
    //             }
    //         }
    //     }
    // });

    let addr = format!("{}:{}", bind, port);
    println!("listening on {}", addr);

    let listener = TcpListener::bind(&addr).await?;

    // TcpListener in tokio has accept() instead of incoming(), plus it is not an iterator
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let store = Arc::clone(&store);
                let aof = aof.as_ref().map(Arc::clone);
                tokio::task::spawn(handle_client(stream, store, aof));
            }
            Err(e) => eprintln!("connection failed: {}", e),
        }
    }
}
