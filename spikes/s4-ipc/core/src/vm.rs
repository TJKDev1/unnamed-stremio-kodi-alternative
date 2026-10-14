//! T0.S4.1: the toy view-model (home screen: rows of cards) and the intents the UI can send.
//!
//! Field order here is the wire order for postcard, and the web decoder
//! (`web/src/decode/postcard.ts`) mirrors it by hand. A real schema would generate both.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Home {
    pub rows: Vec<Row>,
}

/// One catalog row, filled in as its addon answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub id: String,
    pub title: String,
    pub loading: bool,
    pub cards: Vec<Card>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub title: String,
    pub poster: String,
    pub year: Option<u16>,
    /// Watch progress in percent.
    pub progress: Option<u8>,
    pub badges: Vec<String>,
}

/// What the UI asks the core to do. Sent as JSON: intents are small and rare.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    /// Mark a card watched: progress 100 and a "Watched" badge.
    MarkWatched { card: String },
    /// Append the next page of a row's catalog.
    LoadMore { row: String },
    /// Hide a row.
    HideRow { row: String },
}

/// Owns the view-model; addon results and intents mutate it. Patches are computed from
/// snapshots of it, once per frame (see `session`).
#[derive(Debug, Default)]
pub struct Core {
    pub home: Home,
    pages: Vec<(String, u32)>,
}

impl Core {
    pub fn dispatch(&mut self, intent: &Intent) {
        match intent {
            Intent::MarkWatched { card } => {
                for c in self.home.rows.iter_mut().flat_map(|r| r.cards.iter_mut()) {
                    if &c.id == card {
                        c.progress = Some(100);
                        if !c.badges.iter().any(|b| b == "Watched") {
                            c.badges.push("Watched".into());
                        }
                    }
                }
            }
            Intent::LoadMore { row } => {
                let page = match self.pages.iter_mut().find(|(r, _)| r == row) {
                    Some((_, p)) => {
                        *p += 1;
                        *p
                    }
                    None => {
                        self.pages.push((row.clone(), 1));
                        1
                    }
                };
                if let Some(r) = self.home.rows.iter_mut().find(|r| &r.id == row) {
                    for n in 0..10 {
                        r.cards.push(card(&format!("{row}-p{page}-{n}"), n));
                    }
                }
            }
            Intent::HideRow { row } => self.home.rows.retain(|r| &r.id != row),
        }
    }
}

/// A plausible card; `n` varies the fields.
pub fn card(id: &str, n: u32) -> Card {
    let badges = ["4K", "HDR", "DV", "Atmos", "Remux"];
    Card {
        id: id.to_string(),
        title: format!("Title {id}"),
        poster: format!("https://images.example/posters/w342/{id}.webp"),
        year: if n.is_multiple_of(7) {
            None
        } else {
            Some(1990 + (n % 35) as u16)
        },
        progress: if n.is_multiple_of(3) {
            Some((n * 13 % 100) as u8)
        } else {
            None
        },
        badges: (0..n % 3)
            .map(|i| badges[((n + i) % 5) as usize].to_string())
            .collect(),
    }
}
