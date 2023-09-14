use std::collections::HashMap;
use primitive_types::{H160, H256};
use serde::{Serialize, Deserialize};
use crate::types::Address;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StackItem {

    #[serde(rename = "Any")]
    Any {
        value: Option<serde_json::Value>,
    },

    #[serde(rename = "Pointer")]
    Pointer {
        value: i64
    },

    #[serde(rename = "Boolean")]
    Boolean {
        value: bool
    },

    #[serde(rename = "Integer")]
    Integer {
        value: i64
    },

    #[serde(rename = "ByteString")]
    ByteString {
        value: String, // hex encoded
    },

    #[serde(rename = "Buffer")]
    Buffer {
        value: String, // hex encoded
    },

    #[serde(rename = "Array")]
    Array {
        value: Vec<StackItem>,
    },

    #[serde(rename = "Struct")]
    Struct {
        value: Vec<StackItem>
    },

    #[serde(rename = "Map")]
    Map {
        value: Vec<MapEntry>
    },

    #[serde(rename = "InteropInterface")]
    InteropInterface {
        id: String,
        interface: String,
    },

}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapEntry {
    key: StackItem,
    value: StackItem,
}

// Utility methods

impl StackItem {

    fn as_bool(&self) -> Option<bool> {
        match self {
            StackItem::Boolean{value} => Some(*value),
            StackItem::Integer{value} => Some(value != &0),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<String> {
        match self {
            StackItem::ByteString{value} | StackItem::Buffer{value} => {
                hex::decode(value).ok().map(|bytes| String::from_utf8(bytes).ok())?
            },
            StackItem::Integer{value} => Some(value.to_string()),
            StackItem::Boolean{value} => Some(value.to_string()),
            _ => None,
        }
    }

    fn as_bytes(&self) -> Option<Vec<u8>> {
        match self {
            StackItem::ByteString{value} | StackItem::Buffer{value} => hex::decode(value).ok(),
            StackItem::Integer{value} => {
                let mut bytes = value.to_be_bytes().to_vec();
                bytes.reverse();
                Some(bytes)
            },
            _ => None,
        }
    }

    fn as_array(&self) -> Option<Vec<StackItem>> {
        match self {
            StackItem::Array{value} | StackItem::Struct{value} => Some(value.clone()),
            _ => None,
        }
    }

    fn as_int(&self) -> Option<i64> {
        match self {
            StackItem::Integer{value} => Some(*value),
            StackItem::Boolean{value} => Some(if *value {1} else {0}),
            _ => None,
        }
    }

    fn as_map(&self) -> Option<HashMap<StackItem, StackItem>> {
        match self {
            StackItem::Map{value} => {
                let mut map = HashMap::new();
                for entry in value {
                    map.insert(entry.key.clone(), entry.value.clone());
                }
                Some(map)
            },
            _ => None,
        }
    }

    fn as_address(&self) -> Option<Address> {
        self.as_bytes().and_then(|bytes| {
            Address::from_bytes(&bytes).ok()
        })
    }

    fn as_hash160(&self) -> Option<H160> {
        self.as_bytes().and_then(|bytes| {
            H160::from_bytes(&bytes).ok()
        })
    }

    fn as_hash256(&self) -> Option<H256> {
        self.as_bytes().and_then(|bytes| {
            H256::from_bytes(&bytes).ok()
        })
    }
    pub fn len(&self) -> Option<usize> {
        match self {
            StackItem::Array{value} | StackItem::Struct{value} => Some(value.len()),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> Option<bool> {
        self.len().map(|len| len == 0)
    }

    pub fn get(&self, index: usize) -> Option<&StackItem> {
        self.as_array().and_then(|arr| arr.get(index))
    }

    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }

    // ...

}