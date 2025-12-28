use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::sync::{Arc, Mutex};
use crate::engine::AudioEngine;

pub async fn start_server(engine: Arc<Mutex<AudioEngine>>) {
    // Attempt to bind to standard SSIP port
    let listener = match TcpListener::bind("127.0.0.1:6560").await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("SSIP Shim: Could not bind port 6560 (Occupied?): {}", e);
            return;
        }
    };
    
    println!("SSIP Shim listening on 127.0.0.1:6560");

    loop {
        match listener.accept().await {
            Ok((socket, _addr)) => {
                let engine = engine.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket, engine).await {
                        eprintln!("SSIP Connection Error: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("SSIP Accept Error: {}", e),
        }
    }
}

async fn handle_connection(mut socket: tokio::net::TcpStream, engine: Arc<Mutex<AudioEngine>>) -> std::io::Result<()> {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Greeting
    writer.write_all(b"299-SpeechD-NG SSIP Shim\r\n").await?;
    writer.write_all(b"299 OK READY\r\n").await?;

    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            break; // EOF
        }

        let cmd_line = line.trim();
        if cmd_line.is_empty() { continue; }
        
        let cmd_parts: Vec<&str> = cmd_line.split_whitespace().collect();
        let cmd = cmd_parts[0].to_uppercase();

        match cmd.as_str() {
            "SET" => {
                // Determine what we are setting
                // SET SELF CLIENT_NAME|LANGUAGE|...
                // We acknowledge everything to keep client happy
                writer.write_all(b"200 OK\r\n").await?;
            },
            "SPEAK" => {
                // Enter data mode
                writer.write_all(b"202 OK RECEIVING DATA\r\n").await?;
                
                let mut text_buffer = String::new();
                loop {
                    let mut data_line = String::new();
                    if reader.read_line(&mut data_line).await? == 0 { break; }
                    
                    let trimmed = data_line.trim();
                    if trimmed == "." {
                        break;
                    }
                    // Handle ".." escaping? SSIP says: NO, just "." on single line ends it.
                    text_buffer.push_str(&data_line);
                }
                
                // Speak it
                println!("SSIP Speaking: {}", text_buffer);
                if let Ok(e) = engine.lock() {
                    e.speak(&text_buffer, None);
                }
                
                writer.write_all(b"200 OK MESSAGE QUEUED\r\n").await?;
            },
            "QUIT" => {
                writer.write_all(b"231 HAPPY HACKING\r\n").await?;
                return Ok(());
            },
            _ => {
                // Ignore unknown commands with OK to prevent client crash
                // Or maybe 200 OK? Or generic error?
                // Orca might throw if it gets 200 OK for query it expects data for.
                // But for commands like SET, 200 is good.
                // For BLOCK, CANCEL, PAUSE...
                writer.write_all(b"200 OK IGNORED\r\n").await?;
            }
        }
    }
    Ok(())
}
