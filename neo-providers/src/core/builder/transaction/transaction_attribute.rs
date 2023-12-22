use crate::core::transaction::transaction_error::TransactionError;
use neo_codec::{serializable::NeoSerializable, CodecError, Decoder, Encoder};
use neo_types::{Base64Encode, ExternBase64};
use num_bigint::BigInt;
use rustc_serialize::base64::FromBase64;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::hash::Hasher;

use super::oracle_response_code::OracleResponseCode;

#[derive(PartialEq, Hash, Debug, Clone)]
#[serde(tag = "type")]
pub enum TransactionAttribute {
	#[serde(rename = "HighPriority")]
	HighPriority,

	#[serde(rename = "OracleResponse")]
	OracleResponse(OracleResponse),
}

#[derive(Serialize, Deserialize, PartialEq, Hash, Debug, Clone)]
struct OracleResponse {
	pub id: u32,
	pub response_code: OracleResponseCode,
	pub result: String,
}

impl TransactionAttribute {
	pub const MAX_RESULT_SIZE: usize = 0xffff;

	pub fn to_bytes(&self) -> Vec<u8> {
		let mut bytes = vec![];

		match self {
			TransactionAttribute::HighPriority => {
				bytes.push(0x01);
			},
			TransactionAttribute::OracleResponse(OracleResponse { id, response_code, result }) => {
				bytes.push(0x11);
				bytes.extend(&id.to_be_bytes());
				bytes.push(response_code.clone() as u8);
				bytes.extend(result.as_bytes());
			},
		}

		bytes
	}

	pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
		match bytes[0] {
			0x01 => Ok(TransactionAttribute::HighPriority),
			0x11 => {
				if bytes.len() < 9 {
					return Err("Not enough bytes for OracleResponse")
				}
				let mut array = [0; 8];
				let slice_len = bytes[1..9].len();
				array[8 - slice_len..].copy_from_slice(&bytes[1..9]);
				let id = u64::from_be_bytes(array);
				let response_code = OracleResponseCode::try_from(bytes[9]).unwrap();
				let result =
					String::from_utf8(bytes[10..].to_vec()).map_err(|_| "Invalid UTF-8").unwrap();

				Ok(TransactionAttribute::OracleResponse(OracleResponse {
					id: id as u32,
					response_code,
					result,
				}))
			},
			_ => Err("Invalid attribute type byte"),
		}
	}

	pub fn to_json(&self) -> String {
		serde_json::to_string(self).unwrap()
	}
}

impl NeoSerializable for TransactionAttribute {
	type Error = TransactionError;

	fn size(&self) -> usize {
		match self {
			TransactionAttribute::HighPriority => 1,
			TransactionAttribute::OracleResponse(OracleResponse { id, response_code, result }) =>
				1 + 9 + result.len(),
		}
	}

	fn serialize(&self, writer: &mut Encoder) {
		match self {
			TransactionAttribute::HighPriority => {
				writer.write_u8(0x01);
			},
			TransactionAttribute::OracleResponse(OracleResponse { id, response_code, result }) => {
				writer.write_u8(0x11);
				let mut v = id.to_be_bytes();
				v.reverse();
				writer.write(&v);
				writer.write_u8(response_code.clone() as u8);
				writer.write_var_bytes(result.from_base64().unwrap().as_slice());
			},
		}
	}

	fn deserialize(reader: &mut Decoder) -> Result<Self, Self::Error> {
		match reader.read_u8() {
			0x01 => Ok(TransactionAttribute::HighPriority),
			0x11 => {
				let id = reader.read_u32();
				let response_code = OracleResponseCode::try_from(reader.read_u8()?).unwrap();
				let result = reader.read_var_bytes().unwrap().to_base64();

				Ok(TransactionAttribute::OracleResponse(OracleResponse {
					id,
					response_code,
					result,
				}))
			},
			_ => Err(CodecError::InvalidOpCode),
		}
	}

	fn to_array(&self) -> Vec<u8> {
		let mut writer = Encoder::new();
		self.serialize(&mut writer);
		writer.into_bytes()
	}
}