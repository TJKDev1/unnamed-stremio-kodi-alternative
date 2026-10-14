//! Keyed list diff shared by both diff methods: turns old keys into new keys with removes,
//! moves and inserts. Ops apply in order, each to the list as the previous op left it.

use std::collections::HashSet;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    Remove(usize),
    Move {
        from: usize,
        to: usize,
    },
    /// Insert `new[at]` at `at`.
    Insert(usize),
    /// `old[old]` sits at `at` (== its index in `new`); diff the two items in place.
    Keep {
        at: usize,
        old: usize,
    },
}

/// `None` if a key repeats on either side (callers then fall back to a non-keyed diff).
///
/// Items compared in place (`Keep`) are emitted in index order. After `Keep { at }`, later
/// ops only touch indices above `at`, so an in-place diff addressed by `at` stays valid.
pub fn diff<K: Eq + Hash>(old: &[K], new: &[K]) -> Option<Vec<Edit>> {
    if !unique(old) || !unique(new) {
        return None;
    }
    let in_new: HashSet<&K> = new.iter().collect();
    let mut edits = Vec::new();
    // (key, index in old) of the current list.
    let mut cur: Vec<(&K, Option<usize>)> = Vec::with_capacity(new.len());
    for (j, k) in old.iter().enumerate() {
        cur.push((k, Some(j)));
    }
    for j in (0..old.len()).rev() {
        if !in_new.contains(&old[j]) {
            edits.push(Edit::Remove(j));
            cur.remove(j);
        }
    }
    for (i, k) in new.iter().enumerate() {
        if cur.get(i).is_some_and(|(c, _)| *c == k) {
        } else if let Some(j) = cur[i.min(cur.len())..].iter().position(|(c, _)| *c == k) {
            let from = i + j;
            edits.push(Edit::Move { from, to: i });
            let item = cur.remove(from);
            cur.insert(i, item);
        } else {
            edits.push(Edit::Insert(i));
            cur.insert(i, (k, None));
            continue;
        }
        if let Some(old) = cur[i].1 {
            edits.push(Edit::Keep { at: i, old });
        }
    }
    Some(edits)
}

fn unique<K: Eq + Hash>(keys: &[K]) -> bool {
    let mut seen = HashSet::with_capacity(keys.len());
    keys.iter().all(|k| seen.insert(k))
}

/// Applies edits to a plain list (for tests): `Insert` takes the item from `new`.
#[cfg(test)]
pub fn apply<T: Clone>(list: &mut Vec<T>, edits: &[Edit], new: &[T]) {
    for e in edits {
        match *e {
            Edit::Remove(j) => {
                list.remove(j);
            }
            Edit::Move { from, to } => {
                let x = list.remove(from);
                list.insert(to, x);
            }
            Edit::Insert(i) => list.insert(i, new[i].clone()),
            Edit::Keep { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn edits_turn_old_into_new(
            old in proptest::sample::subsequence((0u8..20).collect::<Vec<_>>(), 0..20).prop_shuffle(),
            new in proptest::sample::subsequence((0u8..20).collect::<Vec<_>>(), 0..20).prop_shuffle(),
        ) {
            let edits = diff(&old, &new).unwrap();
            let mut list = old.clone();
            apply(&mut list, &edits, &new);
            prop_assert_eq!(&list, &new);
            for e in &edits {
                if let Edit::Keep { at, old: o } = *e {
                    prop_assert_eq!(old[o], new[at]);
                }
            }
        }
    }

    #[test]
    fn repeated_keys_are_refused() {
        assert!(diff(&[1, 1], &[1]).is_none());
    }
}
