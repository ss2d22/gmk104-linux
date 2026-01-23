use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcMessage {
    Diagnostics {
        errors: u32,
        warnings: u32,
    },
    Activity {
        event: ActivityEvent,
    },
    Break {
        active: bool,
    },
    Status {
        current_mood: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityEvent {
    Typing,
    FileSaved,
    BuildStarted,
    BuildSuccess,
    BuildFailed,
}

pub struct IpcServer {
    socket_path: String,
}

impl IpcServer {
    pub fn new(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.to_string(),
        }
    }

    pub async fn run(&self, tx: mpsc::Sender<IpcMessage>) -> Result<()> {
        let path = Path::new(&self.socket_path);
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        log::info!("IPC server listening on {}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, tx).await {
                            log::warn!("Client error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("Accept error: {}", e);
                }
            }
        }
    }
}

async fn handle_client(stream: UnixStream, tx: mpsc::Sender<IpcMessage>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    log::info!("VS Code extension connected");

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            log::info!("VS Code extension disconnected");
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<IpcMessage>(trimmed) {
            Ok(msg) => {
                log::debug!("Received: {:?}", msg);
                if tx.send(msg).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                log::warn!("Invalid JSON: {} - {}", trimmed, e);
                let error_response = r#"{"error": "invalid json"}"#;
                writer.write_all(error_response.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
        }
    }

    Ok(())
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
