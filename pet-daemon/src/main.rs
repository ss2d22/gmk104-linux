mod config;
mod ipc;
mod state_machine;

use anyhow::Result;
use config::Config;
use ipc::{ActivityEvent, IpcMessage, IpcServer};
use state_machine::{PetMood, PetState};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting panda pet daemon");

    let config = Config::load().unwrap_or_else(|e| {
        log::warn!("Failed to load config: {}, using defaults", e);
        Config::default()
    });

    log::info!("Socket path: {}", config.daemon.socket_path);
    log::info!("Asset path: {}", config.assets.path);
    log::info!("gmk104ctl path: {}", config.daemon.gmk104ctl_path);

    let mood_config = config.to_mood_config();
    let (tx, mut rx) = mpsc::channel::<IpcMessage>(100);

    let socket_path = config.daemon.socket_path.clone();
    tokio::spawn(async move {
        let server = IpcServer::new(&socket_path);
        if let Err(e) = server.run(tx).await {
            log::error!("IPC server error: {}", e);
        }
    });

    let mut state = PetState::new();
    let mut current_displayed_mood: Option<PetMood> = None;
    let mut eval_interval =
        tokio::time::interval(Duration::from_millis(config.daemon.eval_interval_ms));

    log::info!("Daemon running. Waiting for VS Code extension to connect...");

    loop {
        tokio::select! {
            Some(msg) = rx.recv() => {
                match msg {
                    IpcMessage::Diagnostics { errors, warnings } => {
                        log::debug!("Diagnostics: {} errors, {} warnings", errors, warnings);
                        state.update_diagnostics(errors, warnings);
                    }
                    IpcMessage::Activity { event } => {
                        log::debug!("Activity: {:?}", event);
                        state.record_activity();
                        if matches!(event, ActivityEvent::BuildSuccess) {
                            state.record_build_success();
                        }
                    }
                    IpcMessage::Break { active } => {
                        log::info!("Break time: {}", active);
                        state.set_break_time(active);
                    }
                    IpcMessage::Status { .. } => {}
                }
            }

            _ = eval_interval.tick() => {
                let new_mood = state.evaluate_mood(&mood_config);

                if state.should_change_mood(new_mood, mood_config.hysteresis_secs) {
                    state.set_mood(new_mood);

                    if current_displayed_mood != Some(new_mood) {
                        let asset_path = config.asset_path(new_mood.asset_name());
                        if asset_path.exists() {
                            match upload_animation(&asset_path, &config.daemon.gmk104ctl_path) {
                                Ok(()) => {
                                    log::info!("Uploaded {} animation", new_mood.asset_name());
                                    current_displayed_mood = Some(new_mood);
                                }
                                Err(e) => {
                                    log::error!("Failed to upload animation: {}", e);
                                }
                            }
                        } else {
                            log::warn!("Asset not found: {}", asset_path.display());
                        }
                    }
                }
            }
        }
    }
}

fn upload_animation<P: AsRef<Path>>(path: P, gmk104ctl_path: &str) -> Result<()> {
    let path = path.as_ref();
    log::info!("Uploading animation: {}", path.display());

    let output = Command::new(gmk104ctl_path)
        .arg("upload")
        .arg(path)
        .output()?;

    if output.status.success() {
        log::info!("Upload completed successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        log::error!("stdout: {}", stdout);
        log::error!("stderr: {}", stderr);
        anyhow::bail!("gmk104ctl upload failed")
    }
}
