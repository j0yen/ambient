//! ambient CLI — read NDJSON telemetry events from stdin, write NDJSON
//! cue events to stdout.
//!
//! See the library docs (`ambient` crate root) for the event-kind ->
//! voice mapping and the throttle defaults.

#![cfg_attr(not(test), forbid(unsafe_code))]
#![warn(missing_docs)]
// The seven CLI flag fields share a `_secs` suffix on purpose — they
// match the public `--<voice>-secs` flag names and surface the unit.
#![allow(clippy::struct_field_names)]

use ambient::{Throttle, run_stream};
use clap::Parser;
use std::io::{self, BufReader, Write};
use std::process::ExitCode;

/// Telemetry-driven parameter orchestrator for a generative ambient
/// piece.
///
/// Reads newline-delimited JSON events from stdin, writes
/// newline-delimited JSON cues to stdout. Per-voice throttle gaps drop
/// duplicate events within the window so a burst of saves does not
/// produce a burst of chimes.
#[derive(Parser, Debug)]
#[command(name = "ambient", version, about, long_about = None)]
struct Cli {
    /// Minimum seconds between `chime` cues (`file_save`).
    #[arg(long, default_value_t = Throttle::defaults().chime_secs)]
    chime_secs: u64,

    /// Minimum seconds between `piano` cues (`file_create`).
    #[arg(long, default_value_t = Throttle::defaults().piano_secs)]
    piano_secs: u64,

    /// Minimum seconds between `settle` cues (`build_pass`).
    #[arg(long, default_value_t = Throttle::defaults().settle_secs)]
    settle_secs: u64,

    /// Minimum seconds between `grain` cues (`build_fail`).
    #[arg(long, default_value_t = Throttle::defaults().grain_secs)]
    grain_secs: u64,

    /// Minimum seconds between `silence` cues (`idle`).
    #[arg(long, default_value_t = Throttle::defaults().silence_secs)]
    silence_secs: u64,

    /// Minimum seconds between `drift` cues (`high_focus`).
    #[arg(long, default_value_t = Throttle::defaults().drift_secs)]
    drift_secs: u64,

    /// Minimum seconds between `poly` cues (`fragmentation`).
    #[arg(long, default_value_t = Throttle::defaults().poly_secs)]
    poly_secs: u64,
}

impl Cli {
    const fn throttle(&self) -> Throttle {
        Throttle {
            chime_secs: self.chime_secs,
            piano_secs: self.piano_secs,
            settle_secs: self.settle_secs,
            grain_secs: self.grain_secs,
            silence_secs: self.silence_secs,
            drift_secs: self.drift_secs,
            poly_secs: self.poly_secs,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut err_writer = stderr.lock();
    match run_stream(reader, &mut writer, &mut err_writer, cli.throttle()) {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            let _ = io::stderr()
                .lock()
                .write_all(format!("ambient: i/o error: {err}\n").as_bytes());
            ExitCode::from(1)
        }
    }
}
