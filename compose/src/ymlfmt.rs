//! YAML format specification as Rust structs deserialized by serde.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Small helper for turning single-length maps into a tuple.
///
/// Serde deserializes:
/// - signalName:
///     (parameter)
///
/// As a `map<String, YSignal>` with length 1. We then typically have a vector
/// of these, because it's both a sequence element and we still want to have
/// the `':'` after it.
pub fn unmap<T>(map: HashMap<String, T>) -> (String, T) {
    // len should be one because every `- VALUE: val` pair is its own dict
    assert_eq!(map.len(), 1);
    map.into_iter().next().unwrap()
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum YEnumeratedValue {
    Auto(String),
    Exact(HashMap<String, u64>),
}

impl std::fmt::Debug for YEnumeratedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto(_) => write!(f, "(auto)"),
            Self::Exact(map) => write!(f, "{}", unmap(map.clone()).0),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct YSignal {
    pub width: Option<u32>,

    pub start_bit: Option<u32>,

    pub description: Option<String>,

    pub scale: Option<f32>,
    pub unit: Option<String>,

    #[serde(default)]
    pub enumerated_values: Vec<YEnumeratedValue>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct YMessage {
    pub id: u32,

    pub cycletime: Option<u32>,

    pub signals: Vec<HashMap<String, YSignal>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YNode {
    pub messages: Vec<HashMap<String, YMessage>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YDesc {
    pub nodes: Vec<HashMap<String, YNode>>,
}
