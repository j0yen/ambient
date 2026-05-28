# ambient

Telemetry-driven parameter orchestrator for a generative ambient piece.

A Rust CLI that ingests an NDJSON laptop-telemetry stream (file saves,
build pass/fail, idle periods, sustained focus, context fragmentation)
and emits an NDJSON parameter-bus cue stream for an external audio
engine to consume. The downstream engine (Sonic Pi, SuperCollider,
etc.), the sample library, the daily `.wav` recorder, and the annual
album ritual are all out of scope here — `ambient` is the **orchestrator
slice only** (PRD Phase 1).

Per [PRD-ambient-compositions][prd] §1: software work is silent and
visually homogeneous, so days bleed together. A generative ambient
piece driven by your own laptop telemetry gives the workday auditory
texture and yields a daily archive — an audible diary of work.

[prd]: https://github.com/j0yen/autobuilder/blob/main/PRD-ambient-compositions.md

```text
[ telemetry stream (NDJSON) ]  >  ambient  >  [ parameter-bus cues (NDJSON) ]
                                                       |
                                                       v
                                                [ Sonic Pi / SC / OSC bridge ]
```

## Voices

Per PRD §3, each event kind maps to one of seven voices:

| Event           | Voice    | Sonic intent                                          |
|-----------------|----------|-------------------------------------------------------|
| `file_save`     | chime    | soft, key-aligned                                     |
| `file_create`   | piano    | stretched high register                               |
| `build_pass`    | settle   | low harmonic resolve                                  |
| `build_fail`    | grain    | grain decay                                           |
| `idle`          | silence  | actual silence                                        |
| `high_focus`    | drift    | slow tonic slide (steeper with run length)            |
| `fragmentation` | poly     | polyrhythmic layering (denser with more switches)     |

## Throttling

Each voice has a minimum gap so a burst of one kind doesn't overwhelm
the piece. Defaults (seconds): chime 4, piano 8, settle 30, grain 15,
drift 120, poly 30, silence 300. Override with `--chime-secs N`,
`--piano-secs N`, etc.

## Acceptance criteria (MUST)

- **AC1.** Reads newline-delimited JSON telemetry events from stdin and
  writes newline-delimited JSON cue events to stdout, one cue per
  accepted input event (subject to throttle/silence rules).
- **AC2.** Recognizes the seven PRD §3 event kinds and maps each to a
  distinct voice (chime, piano, settle, grain, silence, drift, poly).
- **AC3.** Throttles each voice with a per-voice minimum gap; defaults
  chime 4 / piano 8 / settle 30 / grain 15 / drift 120 / poly 30 /
  silence 300 seconds. Suppressed events drop silently.
- **AC4.** Malformed input lines (invalid JSON, unknown kind, missing
  required field) are skipped without aborting the run; exits 0 on EOF.
- **AC5.** Per-voice throttle gap is configurable via CLI flags.
- **AC6.** Emitted cue lines are valid JSON objects carrying at minimum
  `{voice, kind, unix}` and pass `serde_json::from_str::<Value>`
  round-trip.

## Install

```sh
cargo install --path .
```

Requires Rust 1.85 or later.

## Usage

```sh
# Synthetic telemetry (one NDJSON event per line):
cat <<'EOF' | ambient
{"kind":"file_save","unix":100}
{"kind":"file_save","unix":101}
{"kind":"build_pass","unix":110}
{"kind":"high_focus","unix":200,"run_seconds":3600}
EOF

# Real wiring (sketch — you supply the audio engine):
ctrace tail-events | ambient | osc-bridge --target 127.0.0.1:4559

# Tighten the chime throttle to 1s for a dense session:
ambient --chime-secs 1 < events.ndjson > cues.ndjson
```

## Build & test

```sh
cargo build --release
cargo test
```

14 tests pass at iter-1 (one per acceptance criterion, plus parser /
throttle / mapping micro-tests).

## Out of scope (deferred to future phases)

- The audio engine itself (Sonic Pi script / SuperCollider patch).
- Sample library curation.
- Daily `.wav` recorder (PRD Phase 2).
- Annual album ritual (PRD Phase 3).
- OSC bridge process.

## Provenance

This crate was built end-to-end under
[`/autobuilder`][autobuilder]'s 5-stage pipeline (intake → scaffold →
iterate-and-prove → 25-receipt risk gate → postmortem). Receipts live
under `target/autobuilder/receipts/`. See `agent/intent-card.json` for
the 5-Whys-derived intent.

[autobuilder]: https://github.com/j0yen/autobuilder

## License

Dual MIT / Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE`.
