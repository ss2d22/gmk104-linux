use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub thresholds: ThresholdConfig,
    #[serde(default)]
    pub assets: AssetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    #[serde(default = "default_socket_path")]
    pub socket_path: String,
    #[serde(default = "default_eval_interval")]
    pub eval_interval_ms: u64,
    #[serde(default = "default_hysteresis")]
    pub hysteresis_secs: u64,
    #[serde(default = "default_gmk104ctl_path")]
    pub gmk104ctl_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    #[serde(default = "default_angry_errors")]
    pub angry_errors: u32,
    #[serde(default = "default_sad_errors")]
    pub sad_errors: u32,
    #[serde(default = "default_worried_errors")]
    pub worried_errors: u32,
    #[serde(default = "default_worried_warnings")]
    pub worried_warnings: u32,
    #[serde(default = "default_bored_timeout")]
    pub bored_timeout_secs: u64,
    #[serde(default = "default_sleeping_timeout")]
    pub sleeping_timeout_secs: u64,
    #[serde(default = "default_excited_duration")]
    pub excited_duration_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetConfig {
    #[serde(default = "default_asset_path")]
    pub path: String,
}

fn default_socket_path() -> String {
    "/tmp/panda-pet.sock".to_string()
}

fn default_eval_interval() -> u64 {
    1000
}

fn default_hysteresis() -> u64 {
    5
}

fn default_gmk104ctl_path() -> String {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("gmk104ctl");
            if sibling.exists() {
                return sibling.to_string_lossy().to_string();
            }
        }
    }
    "gmk104ctl".to_string()
}

fn default_angry_errors() -> u32 {
    10
}

fn default_sad_errors() -> u32 {
    5
}

fn default_worried_errors() -> u32 {
    1
}

fn default_worried_warnings() -> u32 {
    10
}

fn default_bored_timeout() -> u64 {
    300
}

fn default_sleeping_timeout() -> u64 {
    900
}

fn default_excited_duration() -> u64 {
    30
}

fn default_asset_path() -> String {
    if let Some(proj_dirs) = ProjectDirs::from("com", "panda-pet", "panda-pet") {
        proj_dirs
            .data_dir()
            .join("assets")
            .to_string_lossy()
            .to_string()
    } else {
        "~/.config/panda-pet/assets".to_string()
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
            eval_interval_ms: default_eval_interval(),
            hysteresis_secs: default_hysteresis(),
            gmk104ctl_path: default_gmk104ctl_path(),
        }
    }
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            angry_errors: default_angry_errors(),
            sad_errors: default_sad_errors(),
            worried_errors: default_worried_errors(),
            worried_warnings: default_worried_warnings(),
            bored_timeout_secs: default_bored_timeout(),
            sleeping_timeout_secs: default_sleeping_timeout(),
            excited_duration_secs: default_excited_duration(),
        }
    }
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            path: default_asset_path(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            daemon: DaemonConfig::default(),
            thresholds: ThresholdConfig::default(),
            assets: AssetConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn config_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "panda-pet", "panda-pet") {
            proj_dirs.config_dir().join("config.toml")
        } else {
            PathBuf::from("~/.config/panda-pet/config.toml")
        }
    }

    pub fn asset_path(&self, mood_name: &str) -> PathBuf {
        let base = shellexpand::tilde(&self.assets.path);
        PathBuf::from(base.as_ref()).join(format!("{}.gif", mood_name))
    }

    pub fn to_mood_config(&self) -> crate::state_machine::MoodConfig {
        crate::state_machine::MoodConfig {
            angry_errors: self.thresholds.angry_errors,
            sad_errors: self.thresholds.sad_errors,
            worried_errors: self.thresholds.worried_errors,
            worried_warnings: self.thresholds.worried_warnings,
            bored_timeout_secs: self.thresholds.bored_timeout_secs,
            sleeping_timeout_secs: self.thresholds.sleeping_timeout_secs,
            excited_duration_secs: self.thresholds.excited_duration_secs,
            hysteresis_secs: self.daemon.hysteresis_secs,
        }
    }
}
