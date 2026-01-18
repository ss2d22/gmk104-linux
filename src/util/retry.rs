use log::warn;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub backoff_factor: f64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            backoff_factor: 2.0,
            max_delay_ms: 2000,
        }
    }
}

impl RetryConfig {
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    pub fn with_delay(mut self, initial_ms: u64) -> Self {
        self.initial_delay_ms = initial_ms;
        self
    }

    pub fn with_backoff(mut self, factor: f64) -> Self {
        self.backoff_factor = factor;
        self
    }

    pub fn execute<T, E, F>(&self, mut op: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
        E: std::fmt::Display,
    {
        let mut delay = self.initial_delay_ms;

        for attempt in 1..=self.max_attempts {
            match op() {
                Ok(result) => return Ok(result),
                Err(e) if attempt < self.max_attempts => {
                    warn!(
                        "Attempt {}/{} failed: {}, retrying in {}ms",
                        attempt, self.max_attempts, e, delay
                    );
                    thread::sleep(Duration::from_millis(delay));
                    delay = ((delay as f64) * self.backoff_factor) as u64;
                    delay = delay.min(self.max_delay_ms);
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    pub fn execute_option<T, F>(&self, mut op: F) -> Option<T>
    where
        F: FnMut() -> Option<T>,
    {
        let mut delay = self.initial_delay_ms;

        for attempt in 1..=self.max_attempts {
            match op() {
                Some(result) => return Some(result),
                None if attempt < self.max_attempts => {
                    warn!(
                        "Attempt {}/{} returned None, retrying in {}ms",
                        attempt, self.max_attempts, delay
                    );
                    thread::sleep(Duration::from_millis(delay));
                    delay = ((delay as f64) * self.backoff_factor) as u64;
                    delay = delay.min(self.max_delay_ms);
                }
                None => return None,
            }
        }

        None
    }
}

pub fn delay_ms(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}
