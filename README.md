# ambient

A Rust CLI that turns a stream of laptop-telemetry events into a stream of cues for a generative ambient piece. Telemetry in, parameter-bus cues out — both NDJSON, one event per line.

## Why it exists

Software work is silent and looks the same every day, so the days blur together. The idea here is to give the workday an auditory texture by driving an ambient piece from your own activity — a file save, a build passing, an hour of unbroken focus, a morning spent thrashing between contexts. Each becomes a sound, and the result is an audible diary: you can hear, after the fact, what a day of work sounded like.

`ambient` is the orchestrator slice of that, and only that. It reads telemetry and decides what should make a sound and when; it does not make the sound. The audio engine (Sonic Pi, SuperCollider, an OSC bridge), the sample library, the daily recorder — all of that is downstream and out of scope. The boundary is deliberate: cues are plain NDJSON, so any engine that can read a line of JSON can play them.

```text
[ telemetry stream (NDJSON) ]  →  ambient  →  [ parameter-bus cues (NDJSON) ]
                                                       │
                                                       ▼
                                              [ Sonic Pi / SuperCollider / OSC bridge ]
```

## Install

```sh
cargo install --path .
```

Requires Rust 1.85 or later.

## Quickstart

Pipe NDJSON telemetry in, read NDJSON cues out:

```sh
cat <<'EOF' | ambient
{"kind":"file_save","unix":100}
{"kind":"file_save","unix":101}
{"kind":"build_pass","unix":110}
{"kind":"high_focus","unix":200,"run_seconds":3600}
EOF
```

Each accepted event yields one cue line carrying at least `{voice, kind, unix}`. Malformed lines — bad JSON, an unknown kind, a missing field — are skipped rather than fatal, so a noisy stream never aborts the run; EOF exits 0. Wired to a real source and sink it looks like:

```sh
ctrace tail-events | ambient | osc-bridge --target 127.0.0.1:4559
ambient --chime-secs 1 < events.ndjson > cues.ndjson   # tighter chime gap for a dense session
```

## Voices

Seven event kinds, each mapped to one voice:

| Event           | Voice    | Sonic intent                                       |
|-----------------|----------|----------------------------------------------------|
| `file_save`     | chime    | soft, key-aligned                                  |
| `file_create`   | piano    | stretched high register                            |
| `build_pass`    | settle   | low harmonic resolve                               |
| `build_fail`    | grain    | grain decay                                        |
| `idle`          | silence  | actual silence                                     |
| `high_focus`    | drift    | slow tonic slide — steeper with run length         |
| `fragmentation` | poly     | polyrhythmic layering — denser with more switches  |

## Throttling

A burst of one kind of event shouldn't drown the piece in one voice, so each voice has a minimum gap; events that arrive inside the gap are dropped silently. The defaults, in seconds:

| Voice | chime | piano | settle | grain | drift | poly | silence |
|-------|-------|-------|--------|-------|-------|------|---------|
| Gap   | 4     | 8     | 30     | 15    | 120   | 30   | 300     |

Each is overridable with `--<voice>-secs N` (`--chime-secs`, `--drift-secs`, and so on).

## Status and scope

This is Phase 1 — the orchestrator. It's complete for what it covers: 14 tests pass, one per behavior guarantee plus parser, throttle, and mapping unit tests. Deferred to later phases and intentionally absent here: the audio engine itself, sample-library curation, the daily `.wav` recorder, the annual album, and the OSC bridge process.

## Provenance

Built end to end through the [autobuilder](https://github.com/j0yen/autobuilder) pipeline (intake → scaffold → iterate-and-prove → risk gate → postmortem). Receipts are under `target/autobuilder/receipts/`; the derived intent is in `agent/intent-card.json`.

## License

Dual-licensed under MIT or Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE`.
