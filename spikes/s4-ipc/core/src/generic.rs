//! T0.S4.2 (a): generic diff over a value tree. Works for any `Serialize` view-model and
//! needs no schema to read, at the cost of converting the whole view-model to a tree every
//! frame. Arrays of objects with unique string `id`s are diffed by key.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::keyed::{self, Edit};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Seg {
    Key(String),
    Idx(u32),
}

pub type Path = Vec<Seg>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Op {
    Set {
        path: Path,
        value: Value,
    },
    Unset {
        path: Path,
    },
    /// Insert into the array at `path`.
    Insert {
        path: Path,
        index: u32,
        value: Value,
    },
    Remove {
        path: Path,
        index: u32,
    },
    Move {
        path: Path,
        from: u32,
        to: u32,
    },
}

pub fn diff(old: &Value, new: &Value) -> Vec<Op> {
    let mut out = Vec::new();
    let mut path = Vec::new();
    diff_at(old, new, &mut path, &mut out);
    out
}

fn diff_at(old: &Value, new: &Value, path: &mut Path, out: &mut Vec<Op>) {
    match (old, new) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, va) in a {
                path.push(Seg::Key(k.clone()));
                match b.get(k) {
                    Some(vb) => diff_at(va, vb, path, out),
                    None => out.push(Op::Unset { path: path.clone() }),
                }
                path.pop();
            }
            for (k, vb) in b {
                if !a.contains_key(k) {
                    path.push(Seg::Key(k.clone()));
                    out.push(Op::Set {
                        path: path.clone(),
                        value: vb.clone(),
                    });
                    path.pop();
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => diff_array(a, b, path, out),
        _ if old == new => {}
        _ => out.push(Op::Set {
            path: path.clone(),
            value: new.clone(),
        }),
    }
}

fn id(v: &Value) -> Option<&str> {
    v.as_object()?.get("id")?.as_str()
}

fn keys(xs: &[Value]) -> Option<Vec<&str>> {
    xs.iter().map(id).collect()
}

fn diff_array(a: &[Value], b: &[Value], path: &mut Path, out: &mut Vec<Op>) {
    let edits = match (keys(a), keys(b)) {
        (Some(ka), Some(kb)) => keyed::diff(&ka, &kb),
        _ => None,
    };
    match edits {
        Some(edits) => {
            for e in edits {
                match e {
                    Edit::Remove(j) => out.push(Op::Remove {
                        path: path.clone(),
                        index: j as u32,
                    }),
                    Edit::Move { from, to } => out.push(Op::Move {
                        path: path.clone(),
                        from: from as u32,
                        to: to as u32,
                    }),
                    Edit::Insert(i) => out.push(Op::Insert {
                        path: path.clone(),
                        index: i as u32,
                        value: b[i].clone(),
                    }),
                    Edit::Keep { at, old } => {
                        path.push(Seg::Idx(at as u32));
                        diff_at(&a[old], &b[at], path, out);
                        path.pop();
                    }
                }
            }
        }
        None if a.len() == b.len() => {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                path.push(Seg::Idx(i as u32));
                diff_at(x, y, path, out);
                path.pop();
            }
        }
        None => out.push(Op::Set {
            path: path.clone(),
            value: Value::Array(b.to_vec()),
        }),
    }
}

/// Reference apply, for tests (the UI has its own, in TypeScript).
pub fn apply(root: &mut Value, ops: &[Op]) {
    for op in ops {
        match op {
            Op::Set { path, value } => match path.split_last() {
                None => *root = value.clone(),
                Some((last, parent)) => match (at(root, parent), last) {
                    (Value::Object(m), Seg::Key(k)) => {
                        m.insert(k.clone(), value.clone());
                    }
                    (Value::Array(xs), Seg::Idx(i)) => xs[*i as usize] = value.clone(),
                    (v, s) => panic!("set {s:?} on {v}"),
                },
            },
            Op::Unset { path } => match path.split_last() {
                Some((Seg::Key(k), parent)) => {
                    obj(at(root, parent)).remove(k);
                }
                _ => panic!("unset needs a key"),
            },
            Op::Insert { path, index, value } => {
                arr(at(root, path)).insert(*index as usize, value.clone());
            }
            Op::Remove { path, index } => {
                arr(at(root, path)).remove(*index as usize);
            }
            Op::Move { path, from, to } => {
                let xs = arr(at(root, path));
                let x = xs.remove(*from as usize);
                xs.insert(*to as usize, x);
            }
        }
    }
}

fn at<'a>(mut v: &'a mut Value, path: &[Seg]) -> &'a mut Value {
    for s in path {
        v = match (v, s) {
            (Value::Object(m), Seg::Key(k)) => m.get_mut(k).expect("path key"),
            (Value::Array(xs), Seg::Idx(i)) => &mut xs[*i as usize],
            (v, s) => panic!("path {s:?} into {v}"),
        };
    }
    v
}

fn arr(v: &mut Value) -> &mut Vec<Value> {
    v.as_array_mut().expect("array")
}

fn obj(v: &mut Value) -> &mut Map<String, Value> {
    v.as_object_mut().expect("object")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testgen::home;
    use proptest::prelude::*;
    use serde_json::json;

    fn value() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            (0i64..5).prop_map(|n| json!(n)),
            "[ab]{0,2}".prop_map(Value::String),
        ];
        leaf.prop_recursive(3, 24, 4, |inner| {
            prop_oneof![
                proptest::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
                proptest::collection::btree_map("[xyz]", inner.clone(), 0..3)
                    .prop_map(|m| Value::Object(m.into_iter().collect())),
                // Keyed arrays.
                proptest::collection::btree_map("[a-f]", inner, 0..5).prop_map(|m| {
                    Value::Array(
                        m.into_iter()
                            .map(|(k, v)| json!({ "id": k, "v": v }))
                            .collect(),
                    )
                }),
            ]
        })
    }

    proptest! {
        #[test]
        fn apply_diff_round_trips(a in value(), b in value()) {
            let mut x = a.clone();
            apply(&mut x, &diff(&a, &b));
            prop_assert_eq!(x, b);
        }

        #[test]
        fn apply_diff_round_trips_homes(a in home(), b in home()) {
            let (a, b) = (serde_json::to_value(a).unwrap(), serde_json::to_value(b).unwrap());
            let mut x = a.clone();
            apply(&mut x, &diff(&a, &b));
            prop_assert_eq!(x, b);
        }
    }
}
