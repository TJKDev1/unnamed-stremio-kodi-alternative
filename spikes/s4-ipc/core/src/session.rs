//! One benchmark run: a scenario driving the core, a diff method and an encoding.
//! Frames are pulled by the UI (one request per frame), so a run is deterministic and the
//! UI can time the whole round trip with one clock.

use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::scenario::Scenario;
use crate::vm::{Core, Home, Intent};
use crate::wire::{DiffKind, Encoding, encode_generic, encode_typed};
use crate::{generic, typed};

/// A frame is a 20-byte header, then the payload. The header is five little-endian u32s:
/// seq, diff ns, encode ns, op count, request ns. "diff ns" includes taking the snapshot that
/// the next frame diffs against. "request ns" is all the work this request did in the core,
/// including running the scenario's ticks, so the UI can subtract it from the round trip to
/// get the transport. (The app's pushed frames don't need it: see the view-model-ipc spec.)
pub const HEADER: usize = 20;

#[derive(Debug)]
pub struct Session {
    core: Core,
    scenario: Scenario,
    tick: usize,
    diff: DiffKind,
    enc: Encoding,
    prev_home: Home,
    prev_value: Value,
    seq: u32,
    /// Something changed since the last diff. A tick without events doesn't diff at all,
    /// as the app's core wouldn't (it knows when its state changed).
    dirty: bool,
}

impl Session {
    pub fn new(scenario: &str, diff: DiffKind, enc: Encoding) -> Option<Self> {
        let core = Core::default();
        Some(Self {
            prev_value: serde_json::to_value(&core.home).unwrap(),
            prev_home: core.home.clone(),
            core,
            scenario: Scenario::new(scenario)?,
            tick: 0,
            diff,
            enc,
            seq: 0,
            dirty: false,
        })
    }

    pub fn dispatch(&mut self, intent: &Intent) {
        self.core.dispatch(intent);
        self.dirty = true;
    }

    /// Runs ticks until one changes the view-model and returns that frame's patch.
    /// Several changes within one tick become one patch (batched per frame).
    /// `None` once the scenario is over and nothing is pending.
    pub fn next_frame(&mut self) -> Option<Vec<u8>> {
        let request = Instant::now();
        loop {
            let ended = self.tick >= self.scenario.len();
            if !ended {
                self.dirty |= self.scenario.run_tick(self.tick, &mut self.core);
                self.tick += 1;
            }
            if !self.dirty {
                if ended {
                    return None;
                }
                continue;
            }
            self.dirty = false;
            let mut out = vec![0; HEADER];
            let t0 = Instant::now();
            let (n, t1) = match self.diff {
                DiffKind::Typed => {
                    let ops = typed::diff(&self.prev_home, &self.core.home);
                    if !ops.is_empty() {
                        self.prev_home.clone_from(&self.core.home);
                    }
                    let t1 = Instant::now();
                    encode_typed(self.enc, &ops, &mut out);
                    (ops.len(), t1)
                }
                DiffKind::Generic => {
                    let next = serde_json::to_value(&self.core.home).unwrap();
                    let ops = generic::diff(&self.prev_value, &next);
                    self.prev_value = next;
                    let t1 = Instant::now();
                    encode_generic(self.enc, &ops, &mut out);
                    (ops.len(), t1)
                }
            };
            let t2 = Instant::now();
            if n == 0 {
                if ended {
                    return None;
                }
                continue;
            }
            self.seq += 1;
            let ns = |d: std::time::Duration| d.as_nanos().min(u32::MAX as u128) as u32;
            for (i, v) in [
                self.seq,
                ns(t1 - t0),
                ns(t2 - t1),
                n as u32,
                ns(request.elapsed()),
            ]
            .into_iter()
            .enumerate()
            {
                out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            return Some(out);
        }
    }

    pub fn home(&self) -> &Home {
        &self.core.home
    }
}

/// Control messages from the UI. Over WebSocket they arrive as JSON text; the Tauri shell
/// maps its commands onto the same handler.
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "lowercase")]
pub enum Command {
    Start {
        scenario: String,
        diff: DiffKind,
        enc: Encoding,
    },
    Next,
    Dispatch {
        intent: Intent,
    },
    Final,
    Results {
        data: Value,
    },
}

#[derive(Debug, Serialize)]
pub enum Reply {
    /// Nothing to send back (dispatch).
    None,
    Text(String),
    /// A frame; empty when the run is over.
    Binary(Vec<u8>),
    Results(Value),
}

pub fn handle(cmd: Command, session: &mut Option<Session>) -> Reply {
    match cmd {
        Command::Start {
            scenario,
            diff,
            enc,
        } => {
            *session = Session::new(&scenario, diff, enc);
            Reply::Text(
                if session.is_some() {
                    "ok"
                } else {
                    "unknown scenario"
                }
                .into(),
            )
        }
        Command::Next => Reply::Binary(
            session
                .as_mut()
                .and_then(Session::next_frame)
                .unwrap_or_default(),
        ),
        Command::Dispatch { intent } => {
            if let Some(s) = session {
                s.dispatch(&intent);
            }
            Reply::None
        }
        Command::Final => Reply::Text(
            session
                .as_ref()
                .map(|s| serde_json::to_string(s.home()).unwrap())
                .unwrap_or_default(),
        ),
        Command::Results { data } => Reply::Results(data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::SCENARIOS;
    use crate::wire::DIFFS;

    /// Rust-side replay of every combination ends in the core's own state.
    #[test]
    fn frames_rebuild_the_view_model() {
        for name in SCENARIOS {
            for diff in DIFFS {
                let mut s = Session::new(name, diff, Encoding::Json).unwrap();
                let (mut home, mut value) = (
                    Home::default(),
                    serde_json::to_value(Home::default()).unwrap(),
                );
                let mut frames = 0;
                while let Some(f) = s.next_frame() {
                    frames += 1;
                    if frames == 30 {
                        s.dispatch(&Intent::LoadMore {
                            row: "addon-1".into(),
                        });
                    }
                    match diff {
                        DiffKind::Typed => typed::apply(
                            &mut home,
                            &serde_json::from_slice::<Vec<typed::Op>>(&f[HEADER..]).unwrap(),
                        ),
                        DiffKind::Generic => generic::apply(
                            &mut value,
                            &serde_json::from_slice::<Vec<generic::Op>>(&f[HEADER..]).unwrap(),
                        ),
                    }
                }
                if diff == DiffKind::Generic {
                    home = serde_json::from_value(value).unwrap();
                }
                assert_eq!(&home, s.home(), "{name} {diff:?}");
                assert!(frames > 2, "{name}: {frames} frames");
            }
        }
    }
}
