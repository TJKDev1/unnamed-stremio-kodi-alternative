//! Deterministic load: what happens to the home screen on each 60 Hz tick.
//!
//! - `stream` (the budget's "typical load"): 10 addon rows appear and fill in chunks of
//!   cards, some rows ranked (cards land mid-row), progress and badge updates, a row moves
//!   to the top and one disappears. 1.5 s of ticks.
//! - `refresh` (worst case): a full 10 × 50 home in one frame, then two frames that reorder
//!   every row and touch a third of the cards.

use crate::vm::{Card, Core, Home, Intent, Row, card};

#[derive(Clone, Debug)]
enum Event {
    AddRow {
        id: String,
    },
    Cards {
        row: String,
        from: u32,
        count: u32,
        ranked: bool,
        last: bool,
    },
    Progress,
    Badge,
    MoveToTop {
        row: String,
    },
    Intent(Intent),
    FullHome,
    Shuffle,
}

#[derive(Debug)]
pub struct Scenario {
    ticks: Vec<Vec<Event>>,
    rng: Rng,
}

pub const SCENARIOS: [&str; 2] = ["stream", "refresh"];

impl Scenario {
    pub fn new(name: &str) -> Option<Self> {
        let mut rng = Rng(0x5eed_5eed_1234_5678);
        let mut ticks: Vec<Vec<Event>> = Vec::new();
        let mut at = |tick: usize, e: Event| {
            if ticks.len() <= tick {
                ticks.resize(tick + 1, Vec::new());
            }
            ticks[tick].push(e);
        };
        match name {
            "stream" => {
                for i in 0..10u32 {
                    let id = format!("addon-{i}");
                    let mut tick = 1 + rng.below(20) as usize;
                    at(tick, Event::AddRow { id: id.clone() });
                    let ranked = i % 3 == 0;
                    let mut from = 0;
                    for chunk in 0..3 {
                        tick += 2 + rng.below(25) as usize;
                        let count = if chunk == 2 {
                            50 - from
                        } else {
                            10 + rng.below(10)
                        };
                        at(
                            tick,
                            Event::Cards {
                                row: id.clone(),
                                from,
                                count,
                                ranked,
                                last: chunk == 2,
                            },
                        );
                        from += count;
                    }
                }
                for tick in 20..90 {
                    if rng.below(10) < 3 {
                        at(tick, Event::Progress);
                    }
                    if rng.below(10) < 1 {
                        at(tick, Event::Badge);
                    }
                }
                at(
                    45,
                    Event::MoveToTop {
                        row: "addon-9".into(),
                    },
                );
                at(
                    60,
                    Event::Intent(Intent::HideRow {
                        row: "addon-4".into(),
                    }),
                );
                at(89, Event::Progress);
            }
            "refresh" => {
                at(0, Event::FullHome);
                at(1, Event::Shuffle);
                at(2, Event::Shuffle);
            }
            _ => return None,
        }
        Some(Self { ticks, rng })
    }

    pub fn len(&self) -> usize {
        self.ticks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ticks.is_empty()
    }

    /// Applies tick `t`'s events to the core.
    pub fn run_tick(&mut self, t: usize, core: &mut Core) {
        let events = self.ticks.get(t).cloned().unwrap_or_default();
        for e in events {
            self.apply(e, core);
        }
    }

    fn apply(&mut self, e: Event, core: &mut Core) {
        let rows = &mut core.home.rows;
        match e {
            Event::AddRow { id } => rows.push(Row {
                title: format!("Catalog {id}"),
                id,
                loading: true,
                cards: Vec::new(),
            }),
            Event::Cards {
                row,
                from,
                count,
                ranked,
                last,
            } => {
                let Some(r) = rows.iter_mut().find(|r| r.id == row) else {
                    return;
                };
                for n in from..from + count {
                    let c = card(&format!("{row}-{n}"), n);
                    if ranked {
                        // Ranked rows keep cards sorted by a score, so new ones land mid-row.
                        let score = |c: &Card| hash(&c.id) % 1000;
                        let pos = r.cards.partition_point(|x| score(x) <= score(&c));
                        r.cards.insert(pos, c);
                    } else {
                        r.cards.push(c);
                    }
                }
                if last {
                    r.loading = false;
                }
            }
            Event::Progress => {
                if let Some(c) = self.pick(rows) {
                    c.progress = Some(self.rng.below(101) as u8);
                }
            }
            Event::Badge => {
                if let Some(c) = self.pick(rows) {
                    match c.badges.iter().position(|b| b == "Cached") {
                        Some(i) => {
                            c.badges.remove(i);
                        }
                        None => c.badges.push("Cached".into()),
                    }
                }
            }
            Event::MoveToTop { row } => {
                if let Some(i) = rows.iter().position(|r| r.id == row) {
                    let r = rows.remove(i);
                    rows.insert(0, r);
                }
            }
            Event::Intent(intent) => core.dispatch(&intent),
            Event::FullHome => core.home = full_home(10, 50),
            Event::Shuffle => {
                self.shuffle(rows);
                for r in rows.iter_mut() {
                    self.shuffle(&mut r.cards);
                    for c in r.cards.iter_mut() {
                        if self.rng.below(3) == 0 {
                            c.progress = Some(self.rng.below(101) as u8);
                        }
                    }
                }
            }
        }
    }

    fn pick<'a>(&mut self, rows: &'a mut [Row]) -> Option<&'a mut Card> {
        let total: usize = rows.iter().map(|r| r.cards.len()).sum();
        if total == 0 {
            return None;
        }
        let n = self.rng.below(total as u32) as usize;
        rows.iter_mut().flat_map(|r| r.cards.iter_mut()).nth(n)
    }

    fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            xs.swap(i, self.rng.below(i as u32 + 1) as usize);
        }
    }
}

pub fn full_home(rows: u32, cards: u32) -> Home {
    Home {
        rows: (0..rows)
            .map(|i| Row {
                id: format!("addon-{i}"),
                title: format!("Catalog addon-{i}"),
                loading: false,
                cards: (0..cards)
                    .map(|n| card(&format!("addon-{i}-{n}"), n))
                    .collect(),
            })
            .collect(),
    }
}

fn hash(s: &str) -> u32 {
    s.bytes()
        .fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

/// xorshift64*: deterministic and dependency-free.
#[derive(Debug)]
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u32) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        ((self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) % n as u64) as u32
    }
}
