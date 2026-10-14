//! T0.S4.2 (b): typed diff, one op per changed field. Hand-written here; in the app a
//! derive macro (or the schema compiler) would generate this, the ops enum and the TS
//! decoder/apply from the view-model structs.
//!
//! Rows and cards are addressed by index; ops apply in order (see `keyed`).

use serde::{Deserialize, Serialize};

use crate::keyed::{self, Edit};
use crate::vm::{Card, Home, Row};

/// Variant order is the postcard wire tag; `web/src/decode/postcard.ts` mirrors it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    RowInsert {
        index: u32,
        row: Row,
    },
    RowRemove {
        index: u32,
    },
    RowMove {
        from: u32,
        to: u32,
    },
    RowTitle {
        row: u32,
        title: String,
    },
    RowLoading {
        row: u32,
        loading: bool,
    },
    /// Replaces all cards of a row (only when card ids repeat, so keyed diff can't run).
    RowCards {
        row: u32,
        cards: Vec<Card>,
    },
    CardInsert {
        row: u32,
        index: u32,
        card: Card,
    },
    CardRemove {
        row: u32,
        index: u32,
    },
    CardMove {
        row: u32,
        from: u32,
        to: u32,
    },
    CardTitle {
        row: u32,
        card: u32,
        title: String,
    },
    CardPoster {
        row: u32,
        card: u32,
        poster: String,
    },
    CardYear {
        row: u32,
        card: u32,
        year: Option<u16>,
    },
    CardProgress {
        row: u32,
        card: u32,
        progress: Option<u8>,
    },
    CardBadges {
        row: u32,
        card: u32,
        badges: Vec<String>,
    },
    /// Replaces the whole home (only when row ids repeat).
    Home {
        home: Home,
    },
}

pub fn diff(old: &Home, new: &Home) -> Vec<Op> {
    let mut out = Vec::new();
    let Some(edits) = keyed::diff(&row_ids(old), &row_ids(new)) else {
        out.push(Op::Home { home: new.clone() });
        return out;
    };
    for e in edits {
        match e {
            Edit::Remove(j) => out.push(Op::RowRemove { index: j as u32 }),
            Edit::Move { from, to } => out.push(Op::RowMove {
                from: from as u32,
                to: to as u32,
            }),
            Edit::Insert(i) => out.push(Op::RowInsert {
                index: i as u32,
                row: new.rows[i].clone(),
            }),
            Edit::Keep { at, old: j } => diff_row(at as u32, &old.rows[j], &new.rows[at], &mut out),
        }
    }
    out
}

fn row_ids(h: &Home) -> Vec<&str> {
    h.rows.iter().map(|r| r.id.as_str()).collect()
}

fn card_ids(r: &Row) -> Vec<&str> {
    r.cards.iter().map(|c| c.id.as_str()).collect()
}

fn diff_row(row: u32, a: &Row, b: &Row, out: &mut Vec<Op>) {
    if a.title != b.title {
        out.push(Op::RowTitle {
            row,
            title: b.title.clone(),
        });
    }
    if a.loading != b.loading {
        out.push(Op::RowLoading {
            row,
            loading: b.loading,
        });
    }
    let Some(edits) = keyed::diff(&card_ids(a), &card_ids(b)) else {
        if a.cards != b.cards {
            out.push(Op::RowCards {
                row,
                cards: b.cards.clone(),
            });
        }
        return;
    };
    for e in edits {
        match e {
            Edit::Remove(j) => out.push(Op::CardRemove {
                row,
                index: j as u32,
            }),
            Edit::Move { from, to } => out.push(Op::CardMove {
                row,
                from: from as u32,
                to: to as u32,
            }),
            Edit::Insert(i) => out.push(Op::CardInsert {
                row,
                index: i as u32,
                card: b.cards[i].clone(),
            }),
            Edit::Keep { at, old: j } => diff_card(row, at as u32, &a.cards[j], &b.cards[at], out),
        }
    }
}

fn diff_card(row: u32, card: u32, a: &Card, b: &Card, out: &mut Vec<Op>) {
    if a.title != b.title {
        out.push(Op::CardTitle {
            row,
            card,
            title: b.title.clone(),
        });
    }
    if a.poster != b.poster {
        out.push(Op::CardPoster {
            row,
            card,
            poster: b.poster.clone(),
        });
    }
    if a.year != b.year {
        out.push(Op::CardYear {
            row,
            card,
            year: b.year,
        });
    }
    if a.progress != b.progress {
        out.push(Op::CardProgress {
            row,
            card,
            progress: b.progress,
        });
    }
    if a.badges != b.badges {
        out.push(Op::CardBadges {
            row,
            card,
            badges: b.badges.clone(),
        });
    }
}

/// Reference apply, for tests (the UI has its own, in TypeScript).
pub fn apply(home: &mut Home, ops: &[Op]) {
    for op in ops.iter().cloned() {
        match op {
            Op::RowInsert { index, row } => home.rows.insert(index as usize, row),
            Op::RowRemove { index } => {
                home.rows.remove(index as usize);
            }
            Op::RowMove { from, to } => {
                let r = home.rows.remove(from as usize);
                home.rows.insert(to as usize, r);
            }
            Op::RowTitle { row, title } => home.rows[row as usize].title = title,
            Op::RowLoading { row, loading } => home.rows[row as usize].loading = loading,
            Op::RowCards { row, cards } => home.rows[row as usize].cards = cards,
            Op::CardInsert { row, index, card } => {
                home.rows[row as usize].cards.insert(index as usize, card)
            }
            Op::CardRemove { row, index } => {
                home.rows[row as usize].cards.remove(index as usize);
            }
            Op::CardMove { row, from, to } => {
                let cards = &mut home.rows[row as usize].cards;
                let c = cards.remove(from as usize);
                cards.insert(to as usize, c);
            }
            Op::CardTitle {
                row,
                card: c,
                title,
            } => card_mut(home, row, c).title = title,
            Op::CardPoster {
                row,
                card: c,
                poster,
            } => card_mut(home, row, c).poster = poster,
            Op::CardYear { row, card: c, year } => card_mut(home, row, c).year = year,
            Op::CardProgress {
                row,
                card: c,
                progress,
            } => card_mut(home, row, c).progress = progress,
            Op::CardBadges {
                row,
                card: c,
                badges,
            } => card_mut(home, row, c).badges = badges,
            Op::Home { home: h } => *home = h,
        }
    }
}

fn card_mut(home: &mut Home, row: u32, card: u32) -> &mut Card {
    &mut home.rows[row as usize].cards[card as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testgen::home;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn apply_diff_round_trips(a in home(), b in home()) {
            let mut x = a.clone();
            apply(&mut x, &diff(&a, &b));
            prop_assert_eq!(x, b);
        }
    }

    #[test]
    fn unchanged_home_has_no_ops() {
        let h = crate::scenario::full_home(3, 5);
        assert!(diff(&h, &h).is_empty());
    }
}
