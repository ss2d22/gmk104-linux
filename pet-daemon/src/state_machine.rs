use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PetMood {
    Happy,
    Worried,
    Sad,
    Angry,
    Excited,
    Bored,
    Sleeping,
    Eating,
}

impl PetMood {
    pub fn asset_name(&self) -> &'static str {
        match self {
            PetMood::Happy => "happy",
            PetMood::Worried => "worried",
            PetMood::Sad => "sad",
            PetMood::Angry => "angry",
            PetMood::Excited => "excited",
            PetMood::Bored => "bored",
            PetMood::Sleeping => "sleeping",
            PetMood::Eating => "eating",
        }
    }
}

pub struct PetState {
    pub current_mood: PetMood,
    pub last_mood_change: Instant,
    pub last_activity: Instant,
    pub last_build_success: Option<Instant>,
    pub error_count: u32,
    pub warning_count: u32,
    pub is_break_time: bool,
    pub vscode_connected: bool,
}

impl Default for PetState {
    fn default() -> Self {
        Self::new()
    }
}

impl PetState {
    pub fn new() -> Self {
        Self {
            current_mood: PetMood::Sleeping,
            last_mood_change: Instant::now(),
            last_activity: Instant::now(),
            last_build_success: None,
            error_count: 0,
            warning_count: 0,
            is_break_time: false,
            vscode_connected: false,
        }
    }

    pub fn update_diagnostics(&mut self, errors: u32, warnings: u32) {
        self.error_count = errors;
        self.warning_count = warnings;
        self.last_activity = Instant::now();
        self.vscode_connected = true;
    }

    pub fn record_activity(&mut self) {
        self.last_activity = Instant::now();
        self.vscode_connected = true;
    }

    pub fn record_build_success(&mut self) {
        self.last_build_success = Some(Instant::now());
        self.last_activity = Instant::now();
    }

    pub fn set_break_time(&mut self, active: bool) {
        self.is_break_time = active;
        if active {
            self.last_activity = Instant::now();
        }
    }

    pub fn disconnect_vscode(&mut self) {
        self.vscode_connected = false;
        self.error_count = 0;
        self.warning_count = 0;
    }

    pub fn evaluate_mood(&self, config: &MoodConfig) -> PetMood {
        if let Some(success_time) = self.last_build_success {
            if success_time.elapsed() < Duration::from_secs(config.excited_duration_secs) {
                return PetMood::Excited;
            }
        }

        if self.error_count >= config.angry_errors {
            return PetMood::Angry;
        }

        if self.error_count >= config.sad_errors {
            return PetMood::Sad;
        }

        if self.error_count >= config.worried_errors || self.warning_count >= config.worried_warnings {
            return PetMood::Worried;
        }

        if self.is_break_time {
            return PetMood::Eating;
        }

        let idle_secs = self.last_activity.elapsed().as_secs();

        if idle_secs >= config.sleeping_timeout_secs {
            return PetMood::Sleeping;
        }

        if idle_secs >= config.bored_timeout_secs {
            return PetMood::Bored;
        }

        PetMood::Happy
    }

    pub fn should_change_mood(&self, new_mood: PetMood, hysteresis_secs: u64) -> bool {
        if self.current_mood == new_mood {
            return false;
        }
        self.last_mood_change.elapsed() >= Duration::from_secs(hysteresis_secs)
    }

    pub fn set_mood(&mut self, mood: PetMood) {
        if self.current_mood != mood {
            log::info!("Mood changing: {:?} -> {:?}", self.current_mood, mood);
            self.current_mood = mood;
            self.last_mood_change = Instant::now();
        }
    }
}

#[derive(Debug, Clone)]
pub struct MoodConfig {
    pub angry_errors: u32,
    pub sad_errors: u32,
    pub worried_errors: u32,
    pub worried_warnings: u32,
    pub bored_timeout_secs: u64,
    pub sleeping_timeout_secs: u64,
    pub excited_duration_secs: u64,
    pub hysteresis_secs: u64,
}

impl Default for MoodConfig {
    fn default() -> Self {
        Self {
            angry_errors: 10,
            sad_errors: 5,
            worried_errors: 1,
            worried_warnings: 10,
            bored_timeout_secs: 300,
            sleeping_timeout_secs: 900,
            excited_duration_secs: 30,
            hysteresis_secs: 5,
        }
    }
}
