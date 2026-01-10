use crate::engine::AudioOutput;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

pub async fn start_server(engine: Arc<dyn AudioOutput + Send + Sync>) {
    // Attempt to bind to standard SSIP port with retries
    let mut retries = 5;
    let listener = loop {
        match TcpListener::bind("127.0.0.1:6560").await {
            Ok(l) => break Some(l),
            Err(e) => {
                if retries > 0 {
                    eprintln!(
                        "SSIP Shim: Bind failed ({}), retrying in 500ms... ({} attempts left)",
                        e, retries
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    retries -= 1;
                } else {
                    eprintln!("SSIP Shim: Could not bind port 6560 after retries: {}", e);
                    break None;
                }
            }
        }
    };

    let listener = match listener {
        Some(l) => l,
        None => return,
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

async fn handle_connection(
    mut socket: tokio::net::TcpStream,
    engine: Arc<dyn AudioOutput + Send + Sync>,
) -> std::io::Result<()> {
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
        if cmd_line.is_empty() {
            continue;
        }

        let cmd_parts: Vec<&str> = cmd_line.split_whitespace().collect();
        let cmd = cmd_parts[0].to_uppercase();

        match cmd.as_str() {
            "SET" => {
                // Determine what we are setting
                // SET SELF CLIENT_NAME|LANGUAGE|...
                // We acknowledge everything to keep client happy
                writer.write_all(b"200 OK\r\n").await?;
            }
            "SPEAK" => {
                // Enter data mode
                writer.write_all(b"202 OK RECEIVING DATA\r\n").await?;

                let mut text_buffer = String::new();
                loop {
                    let mut data_line = String::new();
                    if reader.read_line(&mut data_line).await? == 0 {
                        break;
                    }

                    let trimmed = data_line.trim();
                    if trimmed == "." {
                        break;
                    }
                    // Handle ".." escaping? SSIP says: NO, just "." on single line ends it.
                    text_buffer.push_str(&data_line);
                }

                // Speak it
                println!("SSIP Speaking: {}", text_buffer);
                engine.speak(&text_buffer, None);

                writer.write_all(b"200 OK MESSAGE QUEUED\r\n").await?;
            }
            "QUIT" => {
                writer.write_all(b"231 HAPPY HACKING\r\n").await?;
                return Ok(());
            }
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
