//! Lenient deserializers. Real addons disagree on scalar types (`"imdbRating": 7.5` vs
//! `"7.5"`, `"fileIdx": "3"`) and occasionally send one broken item in an otherwise good
//! list. These helpers accept the common variants and drop (and report) bad list items
//! instead of failing the whole response.

use std::cell::RefCell;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// A list item that failed to parse and was dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dropped {
    /// Which list it was in, e.g. `Stream`, plus its index.
    pub item: String,
    pub error: String,
}

thread_local! {
    static DROPPED: RefCell<Option<Vec<Dropped>>> = const { RefCell::new(None) };
}

/// Runs `f` and returns what it returned plus every item a [`vec`] field dropped while it ran.
/// Deserialization is synchronous, so a thread-local collector is enough.
pub(crate) fn collect_dropped<T>(f: impl FnOnce() -> T) -> (T, Vec<Dropped>) {
    let outer = DROPPED.with(|d| d.borrow_mut().replace(Vec::new()));
    let value = f();
    let dropped = DROPPED.with(|d| std::mem::replace(&mut *d.borrow_mut(), outer));
    (value, dropped.unwrap_or_default())
}

fn record(item: String, error: String) {
    DROPPED.with(|d| {
        if let Some(list) = d.borrow_mut().as_mut() {
            list.push(Dropped { item, error });
        }
    });
}

fn short_type_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    name.rsplit("::").next().unwrap_or(name)
}

/// A list where bad items are dropped and reported. `null` and a missing field are an empty list;
/// a non-list value is reported and treated as empty.
pub(crate) fn vec<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let items = match Value::deserialize(d)? {
        Value::Array(items) => items,
        Value::Null => return Ok(Vec::new()),
        other => {
            record(
                format!("{} list", short_type_name::<T>()),
                format!("expected a list, got {}", kind(&other)),
            );
            return Ok(Vec::new());
        }
    };
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.into_iter().enumerate() {
        match serde_json::from_value(item) {
            Ok(v) => out.push(v),
            Err(e) => record(format!("{}[{i}]", short_type_name::<T>()), e.to_string()),
        }
    }
    Ok(out)
}

/// A list of strings where non-string scalars are stringified and anything else is dropped.
pub(crate) fn strings<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Array(items) => items.into_iter().filter_map(scalar_string).collect(),
        Value::String(s) => vec![s],
        _ => Vec::new(),
    })
}

/// Like [`strings`], but keeps the difference between "absent/null" and "empty list".
pub(crate) fn opt_strings<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<Vec<String>>, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Null => None,
        Value::Array(items) => Some(items.into_iter().filter_map(scalar_string).collect()),
        Value::String(s) => Some(vec![s]),
        _ => None,
    })
}

/// A string that some addons send as a number or bool.
pub(crate) fn opt_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(scalar_string(Value::deserialize(d)?))
}

/// An unsigned number that some addons send as a string. Unparseable values become `None`.
pub(crate) fn opt_u64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Number(n) => n
            .as_u64()
            .or_else(|| n.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    })
}

/// A bool that some addons send as `"true"`/`1`. Missing or anything else is `false`.
pub(crate) fn bool<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Bool(b) => b,
        Value::Number(n) => n.as_u64() == Some(1),
        Value::String(s) => s == "true" || s == "1",
        _ => false,
    })
}

fn scalar_string(v: Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a bool",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}
