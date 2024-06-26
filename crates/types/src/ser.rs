//! Utilities for helping with serialization and deserialization of essential types.

use std::collections::BTreeMap;

use essential_types::{
    convert::{bytes_from_word, word_from_bytes},
    ContentAddress, Key, Value, Word,
};
use serde::{de::Visitor, ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};

#[cfg(test)]
mod tests;

struct Ser<'a>(&'a [Word]);
struct Deser(Vec<Word>);
struct MapSer<'a>(&'a BTreeMap<Key, Value>);
struct MapDeser(BTreeMap<Key, Value>);

struct MapVis;
struct OuterMapVis;

impl<'a> Serialize for Ser<'a> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self
            .0
            .iter()
            .copied()
            .flat_map(bytes_from_word)
            .collect::<Vec<_>>();
        essential_types::serde::bytecode::serialize(&bytes, s)
    }
}

impl<'de> Deserialize<'de> for Deser {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(
            essential_types::serde::bytecode::deserialize(d)?
                .chunks_exact(8)
                .map(|chunk| word_from_bytes(chunk.try_into().expect("Always 8 bytes")))
                .collect(),
        ))
    }
}

impl<'a> Serialize for MapSer<'a> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in self.0 {
            m.serialize_entry(&Ser(k), &Ser(v))?;
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for MapDeser {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(MapDeser(d.deserialize_map(MapVis)?))
    }
}

impl<'de> Visitor<'de> for MapVis {
    type Value = BTreeMap<Key, Value>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a map with bytecode keys and values")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut m = BTreeMap::new();

        while let Some((k, v)) = map.next_entry::<Deser, Deser>()? {
            m.insert(k.0, v.0);
        }

        Ok(m)
    }
}

impl<'de> Visitor<'de> for OuterMapVis {
    type Value = BTreeMap<ContentAddress, BTreeMap<Key, Value>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(
            "a map with content address keys and a map with bytecode keys and values as values",
        )
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut m = BTreeMap::new();

        while let Some((k, v)) = map.next_entry::<ContentAddress, MapDeser>()? {
            m.insert(k, v.0);
        }

        Ok(m)
    }
}

/// Serialize a BTreeMap of content addresses as keys and BTreeMaps of bytecode keys and values as values.
pub fn serialize_map<S>(
    map: &BTreeMap<ContentAddress, BTreeMap<Key, Value>>,
    s: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if s.is_human_readable() {
        let mut m = s.serialize_map(Some(map.len()))?;
        for (ca, kvs) in map {
            m.serialize_entry(&ca, &MapSer(kvs))?;
        }
        m.end()
    } else {
        map.serialize(s)
    }
}

/// Deserialize a BTreeMap of content addresses as keys and BTreeMaps of bytecode keys and values as values.
pub fn deserialize_map<'de, D>(
    d: D,
) -> Result<BTreeMap<ContentAddress, BTreeMap<Key, Value>>, D::Error>
where
    D: Deserializer<'de>,
{
    d.deserialize_map(OuterMapVis)
}
