use crate::utils::*;
use primitive_types::{H160, H256};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct NeoGetNep17Transfers {
	pub nep17_transfers: Option<Nep17Transfers>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Nep17Transfers {
	pub sent: Vec<Nep17Transfer>,
	pub received: Vec<Nep17Transfer>,
	#[serde(rename = "address")]
	pub transfer_address: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Nep17Transfer {
	pub timestamp: u64,
	#[serde(rename = "assethash")]
	#[serde(deserialize_with = "deserialize_address")]
	#[serde(serialize_with = "serialize_address")]
	pub asset_hash: H160,
	#[serde(rename = "transferaddress")]
	pub transfer_address: String,
	pub amount: u64,
	#[serde(rename = "blockindex")]
	pub block_index: u32,
	#[serde(rename = "transfernotifyindex")]
	pub transfer_notify_index: u32,
	#[serde(rename = "txhash")]
	#[serde(serialize_with = "serialize_h256")]
	#[serde(deserialize_with = "deserialize_h256")]
	pub tx_hash: H256,
}
