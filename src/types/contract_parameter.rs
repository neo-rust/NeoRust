use crate::{
	contract::{nef_file::NefFile, role_management::Role},
	types::{contract_parameter_type::ContractParameterType, PublicKey},
};
use base64::encode;
use primitive_types::{H160, H256};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha3::Digest;
use std::hash::{Hash, Hasher};
use strum_macros::{Display, EnumString};

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct ContractParameter {
	#[serde(skip_serializing_if = "Option::is_none")]
	name: Option<String>,
	#[serde(rename = "type")]
	typ: ContractParameterType,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub value: Option<ParameterValue>,
}

impl From<&H160> for ContractParameter {
	fn from(value: &H160) -> Self {
		Self::hash160(value)
	}
}

impl From<H160> for ContractParameter {
	fn from(value: H160) -> Self {
		Self::hash160(&value)
	}
}

impl From<u8> for ContractParameter {
	fn from(value: u8) -> Self {
		Self::integer(value as i64)
	}
}

impl From<i32> for ContractParameter {
	fn from(value: i32) -> Self {
		Self::integer(value as i64)
	}
}

impl From<u32> for ContractParameter {
	fn from(value: u32) -> Self {
		Self::integer(value as i64)
	}
}

impl From<u64> for ContractParameter {
	fn from(value: u64) -> Self {
		Self::integer(value as i64)
	}
}

impl From<Role> for ContractParameter {
	fn from(value: Role) -> Self {
		Self::integer(value as i64)
	}
}

impl From<&str> for ContractParameter {
	fn from(value: &str) -> Self {
		Self::string(value.to_string())
	}
}

impl From<usize> for ContractParameter {
	fn from(value: usize) -> Self {
		Self::integer(value as i64)
	}
}

impl From<&[u8]> for ContractParameter {
	fn from(value: &[u8]) -> Self {
		Self::byte_array(value.to_vec())
	}
}

impl From<Vec<u8>> for ContractParameter {
	fn from(value: Vec<u8>) -> Self {
		Self::byte_array(value)
	}
}

impl From<&PublicKey> for ContractParameter {
	fn from(value: &PublicKey) -> Self {
		Self::public_key(value)
	}
}

impl From<&H256> for ContractParameter {
	fn from(value: &H256) -> Self {
		Self::hash256(value)
	}
}

impl From<&Vec<ContractParameter>> for ContractParameter {
	fn from(value: &Vec<ContractParameter>) -> Self {
		Self::array(value.clone())
	}
}

impl From<&[(ContractParameter, ContractParameter)]> for ContractParameter {
	fn from(value: &[(ContractParameter, ContractParameter)]) -> Self {
		Self::map(value.to_vec())
	}
}

impl From<&NefFile> for ContractParameter {
	fn from(value: &NefFile) -> Self {
		Self::string(serde_json::to_string(&value).unwrap())
	}
}

impl From<String> for ContractParameter {
	fn from(value: String) -> Self {
		Self::string(value)
	}
}

impl From<&String> for ContractParameter {
	fn from(value: &String) -> Self {
		Self::string(value.to_string())
	}
}

#[derive(Display, EnumString, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ParameterValue {
	Boolean(bool),
	Integer(i64),
	ByteArray(String),
	String(String),
	Hash160(String),
	Hash256(String),
	PublicKey(String),
	Signature(String),
	Array(Vec<ContractParameter>),
	Map(Vec<serde_json::Value>),
}

impl ContractParameter {
	pub fn new(typ: ContractParameterType) -> Self {
		Self { name: None, typ, value: None }
	}

	pub fn with_value(typ: ContractParameterType, value: ParameterValue) -> Self {
		Self { name: None, typ, value: Some(value) }
	}

	pub fn bool(value: bool) -> Self {
		Self::with_value(ContractParameterType::Boolean, ParameterValue::Boolean(value))
	}

	pub fn integer(value: i64) -> Self {
		Self::with_value(ContractParameterType::Integer, ParameterValue::Integer(value))
	}

	pub fn byte_array(value: Vec<u8>) -> Self {
		let encoded = encode(value);
		Self::with_value(ContractParameterType::ByteArray, ParameterValue::ByteArray(encoded))
	}

	pub fn string(value: String) -> Self {
		Self::with_value(ContractParameterType::String, ParameterValue::String(value))
	}

	// Other helper methods
	pub fn hash160(value: &H160) -> Self {
		Self::with_value(ContractParameterType::H160, ParameterValue::Hash160(value.to_string()))
	}

	pub fn hash256(value: &H256) -> Self {
		Self::with_value(ContractParameterType::H256, ParameterValue::Hash256(value.to_string()))
	}

	pub fn public_key(value: &PublicKey) -> Self {
		Self::with_value(
			ContractParameterType::PublicKey,
			ParameterValue::PublicKey(value.to_string()),
		)
	}

	pub fn signature(value: &str) -> Self {
		Self::with_value(
			ContractParameterType::Signature,
			ParameterValue::Signature(value.to_string()),
		)
	}

	pub fn array(values: Vec<Self>) -> Self {
		Self::with_value(ContractParameterType::Array, ParameterValue::Array(values))
	}

	pub fn map(values: Vec<(Self, Self)>) -> Self {
		let json = values.into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect();

		Self::with_value(ContractParameterType::Map, ParameterValue::Map(json))
	}

	pub fn hash(self) -> Vec<u8> {
		let mut hasher = std::collections::hash_map::DefaultHasher::new();
		Hash::hash(&self, &mut hasher);
		hasher.finish().to_be_bytes().to_vec()
	}
}
