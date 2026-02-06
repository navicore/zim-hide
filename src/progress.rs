//! Progress indicators for long-running operations.
//!
//! Respects verbosity settings - no progress in quiet mode.

use crate::Verbosity;
use indicatif::{ProgressBar, ProgressStyle};

/// A progress bar that respects verbosity settings.
pub struct Progress {
    bar: Option<ProgressBar>,
}

impl Progress {
    /// Create a new progress bar with known total.
    pub fn new(total: u64, verbosity: Verbosity) -> Self {
        let bar = if verbosity.show_status() {
            let pb = ProgressBar::new(total);
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{bar:30.cyan/blue}] {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("#>-"),
            );
            Some(pb)
        } else {
            None
        };
        Self { bar }
    }

    /// Create a spinner for unknown-length operations.
    pub fn spinner(msg: &str, verbosity: Verbosity) -> Self {
        let bar = if verbosity.show_status() {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::with_template("{spinner:.green} {msg}")
                    .unwrap()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
            );
            pb.set_message(msg.to_string());
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            Some(pb)
        } else {
            None
        };
        Self { bar }
    }

    /// Increment the progress bar by one.
    pub fn inc(&self, delta: u64) {
        if let Some(ref bar) = self.bar {
            bar.inc(delta);
        }
    }

    /// Set the current position.
    pub fn set_position(&self, pos: u64) {
        if let Some(ref bar) = self.bar {
            bar.set_position(pos);
        }
    }

    /// Set the message shown on the progress bar.
    pub fn set_message(&self, msg: impl Into<String>) {
        if let Some(ref bar) = self.bar {
            bar.set_message(msg.into());
        }
    }

    /// Finish the progress bar with a message.
    pub fn finish_with_message(&self, msg: impl Into<String>) {
        if let Some(ref bar) = self.bar {
            bar.finish_with_message(msg.into());
        }
    }

    /// Finish and clear the progress bar.
    pub fn finish_and_clear(&self) {
        if let Some(ref bar) = self.bar {
            bar.finish_and_clear();
        }
    }
}
