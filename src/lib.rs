//! ambient — telemetry-driven parameter orchestrator for a generative
//! ambient piece.
//!
//! This crate ships the orchestrator slice described by PRD Phase 1: a
//! pure NDJSON transducer that ingests laptop telemetry events on
//! stdin and emits parameter-bus cue events on stdout.
//!
//! The downstream audio engine (Sonic Pi, `SuperCollider`, etc.) is
//! out of scope; we end at the NDJSON cue stream.
//!
//! # Pipeline
//!
//! ```text
//! NDJSON event line  ->  parse_event_line  ->  Event
//!                                                |
//!                                                v
//!                                            Engine::handle
//!                                                |
//!                                                v
//!                                          Option<Cue>  ->  NDJSON cue line
//! ```
//!
//! Per-voice throttle gaps prevent a burst of incoming events of the
//! same kind from producing a burst of cues; the burst-to-burst
//! transduction is exactly what would make the piece sound like a
//! meditation app (PRD §8).

#![cfg_attr(not(test), forbid(unsafe_code))]
#![warn(missing_docs)]
// `struct_field_names`: the seven throttle fields share a `_secs`
// suffix on purpose — they're CLI flag names, and renaming them would
// hide the unit (seconds) from the public API.
#![allow(clippy::struct_field_names)]

use serde::{Deserialize, Serialize};
use std::fmt;

/// The seven event kinds defined by PRD §3.
///
/// Only the fields the orchestrator uses are carried; extra fields in
/// the input JSON are tolerated and dropped.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// A file was saved.
    FileSave {
        /// Unix timestamp (seconds since epoch).
        unix: i64,
    },
    /// A new file was created.
    FileCreate {
        /// Unix timestamp.
        unix: i64,
    },
    /// A build succeeded.
    BuildPass {
        /// Unix timestamp.
        unix: i64,
    },
    /// A build failed.
    BuildFail {
        /// Unix timestamp.
        unix: i64,
    },
    /// Long idle period.
    Idle {
        /// Unix timestamp.
        unix: i64,
    },
    /// Sustained focus on a single project.
    ///
    /// The longer the run, the steeper the downstream drift slope.
    HighFocus {
        /// Unix timestamp.
        unix: i64,
        /// Duration of the focus run in seconds.
        run_seconds: u64,
    },
    /// Context fragmentation.
    ///
    /// The more switches, the denser the downstream polyrhythmic layer.
    Fragmentation {
        /// Unix timestamp.
        unix: i64,
        /// Number of distinct project switches in the window.
        switch_count: u64,
    },
}

impl Event {
    /// Returns the Unix timestamp carried by every variant.
    #[must_use]
    pub const fn unix(&self) -> i64 {
        match *self {
            Self::FileSave { unix }
            | Self::FileCreate { unix }
            | Self::BuildPass { unix }
            | Self::BuildFail { unix }
            | Self::Idle { unix }
            | Self::HighFocus { unix, .. }
            | Self::Fragmentation { unix, .. } => unix,
        }
    }

    /// The voice this event maps to per the PRD §3 voice table.
    #[must_use]
    pub const fn voice(&self) -> Voice {
        match *self {
            Self::FileSave { .. } => Voice::Chime,
            Self::FileCreate { .. } => Voice::Piano,
            Self::BuildPass { .. } => Voice::Settle,
            Self::BuildFail { .. } => Voice::Grain,
            Self::Idle { .. } => Voice::Silence,
            Self::HighFocus { .. } => Voice::Drift,
            Self::Fragmentation { .. } => Voice::Poly,
        }
    }

    /// The wire-format kind string (matches the input `kind`
    /// discriminator and is round-tripped to the emitted cue).
    #[must_use]
    pub const fn kind_str(&self) -> &'static str {
        match *self {
            Self::FileSave { .. } => "file_save",
            Self::FileCreate { .. } => "file_create",
            Self::BuildPass { .. } => "build_pass",
            Self::BuildFail { .. } => "build_fail",
            Self::Idle { .. } => "idle",
            Self::HighFocus { .. } => "high_focus",
            Self::Fragmentation { .. } => "fragmentation",
        }
    }
}

/// The seven sonic voices the engine emits.
///
/// Each event kind maps to exactly one voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Voice {
    /// Soft, key-aligned chime. Driven by `file_save`.
    Chime,
    /// Stretched high-register piano note. Driven by `file_create`.
    Piano,
    /// Low harmonic resolution. Driven by `build_pass`.
    Settle,
    /// Grain decay. Driven by `build_fail`.
    Grain,
    /// Actual silence. Driven by `idle`.
    Silence,
    /// Slow tonic slide. Driven by `high_focus`.
    Drift,
    /// Polyrhythmic layering. Driven by `fragmentation`.
    Poly,
}

impl Voice {
    /// All seven voices, in declaration order.
    ///
    /// Useful for iteration in tests and CLI help output.
    pub const ALL: [Self; 7] = [
        Self::Chime,
        Self::Piano,
        Self::Settle,
        Self::Grain,
        Self::Silence,
        Self::Drift,
        Self::Poly,
    ];

    /// The wire-format string representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Chime => "chime",
            Self::Piano => "piano",
            Self::Settle => "settle",
            Self::Grain => "grain",
            Self::Silence => "silence",
            Self::Drift => "drift",
            Self::Poly => "poly",
        }
    }

    /// Index into the engine's per-voice last-emit table.
    const fn index(self) -> usize {
        match self {
            Self::Chime => 0,
            Self::Piano => 1,
            Self::Settle => 2,
            Self::Grain => 3,
            Self::Silence => 4,
            Self::Drift => 5,
            Self::Poly => 6,
        }
    }
}

impl fmt::Display for Voice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Per-voice minimum gap, in seconds.
///
/// A new cue for a voice is only emitted if at least this many seconds
/// have elapsed since the previous cue for the same voice; otherwise
/// the event is silently dropped.
///
/// Defaults are carried forward from the PRD §3 prose and the prior
/// hand-built scaffold:
///
/// | voice   | default seconds |
/// |---------|----------------:|
/// | chime   |               4 |
/// | piano   |               8 |
/// | settle  |              30 |
/// | grain   |              15 |
/// | silence |             300 |
/// | drift   |             120 |
/// | poly    |              30 |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Throttle {
    /// Minimum seconds between `chime` cues.
    pub chime_secs: u64,
    /// Minimum seconds between `piano` cues.
    pub piano_secs: u64,
    /// Minimum seconds between `settle` cues.
    pub settle_secs: u64,
    /// Minimum seconds between `grain` cues.
    pub grain_secs: u64,
    /// Minimum seconds between `silence` cues.
    pub silence_secs: u64,
    /// Minimum seconds between `drift` cues.
    pub drift_secs: u64,
    /// Minimum seconds between `poly` cues.
    pub poly_secs: u64,
}

impl Throttle {
    /// Default throttle gaps per the PRD.
    #[must_use]
    pub const fn defaults() -> Self {
        Self {
            chime_secs: 4,
            piano_secs: 8,
            settle_secs: 30,
            grain_secs: 15,
            silence_secs: 300,
            drift_secs: 120,
            poly_secs: 30,
        }
    }

    /// Look up the gap (in seconds) for a given voice.
    #[must_use]
    pub const fn gap_for(&self, voice: Voice) -> u64 {
        match voice {
            Voice::Chime => self.chime_secs,
            Voice::Piano => self.piano_secs,
            Voice::Settle => self.settle_secs,
            Voice::Grain => self.grain_secs,
            Voice::Silence => self.silence_secs,
            Voice::Drift => self.drift_secs,
            Voice::Poly => self.poly_secs,
        }
    }
}

impl Default for Throttle {
    fn default() -> Self {
        Self::defaults()
    }
}

/// An emitted parameter-bus cue.
///
/// Wire format is a JSON object on a single line. `voice`, `kind`, and
/// `unix` are always present; `intensity` carries the per-event
/// intensity payload for `high_focus` (`run_seconds`) and
/// `fragmentation` (`switch_count`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cue {
    /// The voice this cue addresses.
    pub voice: Voice,
    /// The originating event kind (round-tripped from the input).
    pub kind: &'static str,
    /// Unix timestamp from the originating event.
    pub unix: i64,
    /// Optional intensity scalar.
    ///
    /// Present only for `high_focus` and `fragmentation` cues; absent
    /// (skipped during serialization) for the other five voices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intensity: Option<u64>,
}

/// Errors surfaced by `parse_event_line`.
///
/// Callers in the CLI treat all variants as "skip this line and keep
/// going" per AC4.
#[derive(Debug)]
pub enum ParseError {
    /// The line wasn't valid JSON, or didn't deserialize as any of the
    /// seven known kinds, or was missing a required field.
    Decode(serde_json::Error),
    /// The line was empty (no content other than whitespace).
    Empty,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(err) => write!(f, "decode error: {err}"),
            Self::Empty => f.write_str("empty line"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(err) => Some(err),
            Self::Empty => None,
        }
    }
}

/// Parse a single line of NDJSON into an `Event`.
///
/// Empty / whitespace-only lines yield `ParseError::Empty`; invalid
/// JSON or unknown kinds yield `ParseError::Decode`. The CLI loop
/// converts both into a silent skip (with an optional stderr log) so
/// a malformed line never aborts the run (AC4).
///
/// # Errors
///
/// Returns `ParseError` when the line is empty or fails to deserialize
/// into one of the seven `Event` variants.
pub fn parse_event_line(line: &str) -> Result<Event, ParseError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }
    serde_json::from_str::<Event>(trimmed).map_err(ParseError::Decode)
}

/// Streaming orchestrator.
///
/// Holds the throttle configuration and the last-emit timestamp per
/// voice. `Engine::handle` is the only mutation point.
#[derive(Debug, Clone)]
pub struct Engine {
    throttle: Throttle,
    last_emit: [Option<i64>; 7],
}

impl Engine {
    /// Construct an engine with the given throttle gaps.
    ///
    /// No prior emit history; every voice is "ready" until its first
    /// cue.
    #[must_use]
    pub const fn new(throttle: Throttle) -> Self {
        Self { throttle, last_emit: [None; 7] }
    }

    /// The current throttle configuration. Useful for diagnostics.
    #[must_use]
    pub const fn throttle(&self) -> Throttle {
        self.throttle
    }

    /// Handle one event.
    ///
    /// Returns `Some(Cue)` if the event's voice is outside its
    /// throttle window (or has never fired), `None` if the event is
    /// suppressed.
    ///
    /// Throttling honors the event's own `unix` timestamp rather than
    /// wall-clock time, so a replay of an archived event stream
    /// produces the same cue stream it produced live.
    pub fn handle(&mut self, event: &Event) -> Option<Cue> {
        let voice = event.voice();
        let now = event.unix();
        let idx = voice.index();
        let gap = self.throttle.gap_for(voice);
        // gap is in seconds; treat as i64 for arithmetic. We bound the
        // input event timestamps to i64 anyway.
        let gap_i64 = i64::try_from(gap).unwrap_or(i64::MAX);
        // `.get(idx)` over indexing: idx comes from voice.index() and
        // is statically in 0..7 for the 7-slot array, but clippy can't
        // see that. .get is free at runtime and silences the lint.
        let slot = self.last_emit.get(idx).copied().flatten();
        if let Some(last) = slot {
            // saturating_sub keeps a malformed past-event from
            // accidentally satisfying the throttle by going negative.
            if now.saturating_sub(last) < gap_i64 {
                return None;
            }
        }
        if let Some(cell) = self.last_emit.get_mut(idx) {
            *cell = Some(now);
        }
        let intensity = match *event {
            Event::HighFocus { run_seconds, .. } => Some(run_seconds),
            Event::Fragmentation { switch_count, .. } => Some(switch_count),
            _ => None,
        };
        Some(Cue { voice, kind: event.kind_str(), unix: now, intensity })
    }
}

/// Drive an event stream end-to-end.
///
/// Reads NDJSON from `reader`, writes NDJSON cues to `writer`, logs
/// skipped-malformed lines to `err`. Returns the number of cues
/// emitted.
///
/// # Errors
///
/// Returns any I/O error from the underlying reader or writer. Parse
/// errors on individual lines are NOT propagated — they're logged to
/// `err` and the loop continues (AC4).
pub fn run_stream<R, W, E>(
    reader: R,
    mut writer: W,
    mut err: E,
    throttle: Throttle,
) -> std::io::Result<u64>
where
    R: std::io::BufRead,
    W: std::io::Write,
    E: std::io::Write,
{
    let mut engine = Engine::new(throttle);
    let mut emitted: u64 = 0;
    for (lineno, line) in reader.lines().enumerate() {
        let line = line?;
        match parse_event_line(&line) {
            Ok(event) => {
                if let Some(cue) = engine.handle(&event) {
                    // serde_json::to_string on a Cue can only fail for
                    // exotic IO scenarios; surface it as an io::Error
                    // so the caller sees one unified error type.
                    let s = serde_json::to_string(&cue).map_err(std::io::Error::other)?;
                    writer.write_all(s.as_bytes())?;
                    writer.write_all(b"\n")?;
                    emitted = emitted.saturating_add(1);
                }
            }
            Err(ParseError::Empty) => {
                // Silent skip — empty lines are common at EOF.
            }
            Err(ParseError::Decode(decode_err)) => {
                // Log to stderr but don't abort (AC4). Format includes
                // 1-based line number for human debuggability.
                let _ = writeln!(
                    err,
                    "ambient: skipping line {}: {}",
                    lineno.saturating_add(1),
                    decode_err
                );
            }
        }
    }
    writer.flush()?;
    Ok(emitted)
}
