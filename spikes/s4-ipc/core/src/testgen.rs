//! Random homes for property tests: ids come from small pools so diffs hit moves, inserts,
//! removes and in-place edits.

use proptest::prelude::*;

use crate::vm::{Card, Home, Row};

fn card() -> impl Strategy<Value = Card> {
    (
        0u8..12,
        "[ab]{0,2}",
        prop::option::of(1990u16..1993),
        prop::option::of(0u8..3),
        prop::collection::vec(prop::sample::select(vec!["4K", "HDR", "Cached"]), 0..3),
    )
        .prop_map(|(id, title, year, progress, badges)| Card {
            id: format!("c{id}"),
            poster: format!("p{title}"),
            title,
            year,
            progress,
            badges: badges.into_iter().map(String::from).collect(),
        })
}

fn row() -> impl Strategy<Value = Row> {
    (
        0u8..6,
        "[xy]{0,1}",
        any::<bool>(),
        prop::collection::vec(card(), 0..8),
    )
        .prop_map(|(id, title, loading, cards)| Row {
            id: format!("r{id}"),
            title,
            loading,
            cards,
        })
}

/// Ids may repeat, which exercises the non-keyed fallbacks too.
pub fn home() -> impl Strategy<Value = Home> {
    prop::collection::vec(row(), 0..5).prop_map(|rows| Home { rows })
}
