//! T0.S4.3: encodings. JSON and MessagePack are self-describing and decode to the same JS
//! objects; postcard is positional and needs a schema-aware decoder on the JS side.

use serde::ser::{SerializeSeq, SerializeStructVariant, SerializeTuple};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

use crate::{generic, typed};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Json,
    Msgpack,
    Postcard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    Generic,
    Typed,
}

pub const ENCODINGS: [Encoding; 3] = [Encoding::Json, Encoding::Msgpack, Encoding::Postcard];
pub const DIFFS: [DiffKind; 2] = [DiffKind::Generic, DiffKind::Typed];

pub fn encode_typed(enc: Encoding, ops: &[typed::Op], out: &mut Vec<u8>) {
    match enc {
        Encoding::Json => serde_json::to_writer(out, ops).unwrap(),
        // Named: structs as maps, so the JS side gets the same objects as from JSON.
        Encoding::Msgpack => ops
            .serialize(&mut rmp_serde::Serializer::new(out).with_struct_map())
            .unwrap(),
        Encoding::Postcard => {
            out.extend_from_slice(&postcard::to_allocvec(ops).unwrap());
        }
    }
}

pub fn encode_generic(enc: Encoding, ops: &[generic::Op], out: &mut Vec<u8>) {
    match enc {
        Encoding::Json => serde_json::to_writer(out, ops).unwrap(),
        Encoding::Msgpack => ops
            .serialize(&mut rmp_serde::Serializer::new(out).with_struct_map())
            .unwrap(),
        Encoding::Postcard => {
            let ops: Vec<_> = ops.iter().map(Tagged).collect();
            out.extend_from_slice(&postcard::to_allocvec(&ops).unwrap());
        }
    }
}

/// Postcard can't carry untagged values (paths mix strings and numbers; values are any JSON),
/// so generic ops go out with explicit tags. Variant numbers are the wire tags.
struct Tagged<'a, T>(&'a T);

impl Serialize for Tagged<'_, generic::Op> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use generic::Op;
        match self.0 {
            Op::Set { path: p, value } => {
                let mut v = s.serialize_struct_variant("Op", 0, "Set", 2)?;
                v.serialize_field("path", &path(p))?;
                v.serialize_field("value", &Tagged(value))?;
                v.end()
            }
            Op::Unset { path: p } => {
                let mut v = s.serialize_struct_variant("Op", 1, "Unset", 1)?;
                v.serialize_field("path", &path(p))?;
                v.end()
            }
            Op::Insert {
                path: p,
                index,
                value,
            } => {
                let mut v = s.serialize_struct_variant("Op", 2, "Insert", 3)?;
                v.serialize_field("path", &path(p))?;
                v.serialize_field("index", index)?;
                v.serialize_field("value", &Tagged(value))?;
                v.end()
            }
            Op::Remove { path: p, index } => {
                let mut v = s.serialize_struct_variant("Op", 3, "Remove", 2)?;
                v.serialize_field("path", &path(p))?;
                v.serialize_field("index", index)?;
                v.end()
            }
            Op::Move { path: p, from, to } => {
                let mut v = s.serialize_struct_variant("Op", 4, "Move", 3)?;
                v.serialize_field("path", &path(p))?;
                v.serialize_field("from", from)?;
                v.serialize_field("to", to)?;
                v.end()
            }
        }
    }
}

impl Serialize for Tagged<'_, Vec<generic::Seg>> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for seg in self.0 {
            match seg {
                generic::Seg::Key(k) => seq.serialize_element(&SegTag::Key(k))?,
                generic::Seg::Idx(i) => seq.serialize_element(&SegTag::Idx(*i))?,
            }
        }
        seq.end()
    }
}

fn path(p: &Vec<generic::Seg>) -> Tagged<'_, Vec<generic::Seg>> {
    Tagged(p)
}

#[derive(Serialize)]
enum SegTag<'a> {
    Key(&'a str),
    Idx(u32),
}

/// Tags: 0 null, 1 bool, 2 int (zigzag varint), 3 float (f64), 4 string, 5 array, 6 object.
impl Serialize for Tagged<'_, Value> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Null => s.serialize_unit_variant("V", 0, "Null"),
            Value::Bool(b) => s.serialize_newtype_variant("V", 1, "Bool", b),
            Value::Number(n) => match n.as_i64() {
                Some(i) => s.serialize_newtype_variant("V", 2, "Int", &i),
                None => s.serialize_newtype_variant("V", 3, "Float", &n.as_f64().unwrap()),
            },
            Value::String(st) => s.serialize_newtype_variant("V", 4, "Str", st),
            Value::Array(xs) => s.serialize_newtype_variant("V", 5, "Arr", &Seq(xs)),
            Value::Object(m) => s.serialize_newtype_variant("V", 6, "Obj", &Entries(m)),
        }
    }
}

struct Seq<'a>(&'a [Value]);

impl Serialize for Seq<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for v in self.0 {
            seq.serialize_element(&Tagged(v))?;
        }
        seq.end()
    }
}

struct Entries<'a>(&'a serde_json::Map<String, Value>);

impl Serialize for Entries<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for (k, v) in self.0 {
            seq.serialize_element(&Entry(k, v))?;
        }
        seq.end()
    }
}

struct Entry<'a>(&'a str, &'a Value);

impl Serialize for Entry<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut t = s.serialize_tuple(2)?;
        t.serialize_element(self.0)?;
        t.serialize_element(&Tagged(self.1))?;
        t.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::full_home;

    fn ops() -> (Vec<typed::Op>, Vec<generic::Op>) {
        let (a, mut b) = (full_home(2, 3), full_home(3, 4));
        b.rows[0].cards[1].progress = None;
        b.rows.swap(0, 2);
        let ga = serde_json::to_value(&a).unwrap();
        let gb = serde_json::to_value(&b).unwrap();
        (typed::diff(&a, &b), generic::diff(&ga, &gb))
    }

    /// The JS side applies JSON and MessagePack ops with the same code, so both must decode
    /// to the same tree.
    #[test]
    fn msgpack_decodes_like_json() {
        let (t, g) = ops();
        for (json, mp) in [
            (serde_json::to_value(&t).unwrap(), {
                let mut out = Vec::new();
                encode_typed(Encoding::Msgpack, &t, &mut out);
                rmp_serde::from_slice::<Value>(&out).unwrap()
            }),
            (serde_json::to_value(&g).unwrap(), {
                let mut out = Vec::new();
                encode_generic(Encoding::Msgpack, &g, &mut out);
                rmp_serde::from_slice::<Value>(&out).unwrap()
            }),
        ] {
            assert_eq!(json, mp);
        }
    }

    #[test]
    fn typed_postcard_round_trips() {
        let (t, _) = ops();
        let mut out = Vec::new();
        encode_typed(Encoding::Postcard, &t, &mut out);
        assert_eq!(postcard::from_bytes::<Vec<typed::Op>>(&out).unwrap(), t);
    }
}
