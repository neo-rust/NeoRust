use crate::{
	protocol::core::responses::{
		neo_witness::NeoWitness, transaction_attribute::TransactionAttribute,
	},
	types::vm_state::VMState,
	utils::*,
};
use primitive_types::{H160, H256};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use crate::transaction::signers::transaction_signer::TransactionSigner;

#[derive(Serialize, Deserialize, Hash, Debug, Clone)]
pub struct Transaction {
	#[serde(rename = "hash")]
	#[serde(serialize_with = "serialize_h256")]
	#[serde(deserialize_with = "deserialize_h256")]
	pub hash: H256,

	#[serde(rename = "size")]
	pub size: i32,

	#[serde(rename = "version")]
	pub version: i32,

	#[serde(rename = "nonce")]
	pub nonce: i32,

	#[serde(rename = "sender")]
	#[serde(deserialize_with = "deserialize_address")]
	#[serde(serialize_with = "serialize_address")]
	pub sender: H160,

	#[serde(rename = "sysfee")]
	pub sys_fee: String,

	#[serde(rename = "netfee")]
	pub net_fee: String,

	#[serde(rename = "validuntilblock")]
	pub valid_until_block: i32,

	#[serde(rename = "signers")]
	pub signers: Vec<TransactionSigner>,

	#[serde(rename = "attributes")]
	pub attributes: Vec<TransactionAttribute>,

	#[serde(rename = "script")]
	pub script: String,

	#[serde(rename = "witnesses")]
	pub witnesses: Vec<NeoWitness>,

	#[serde(rename = "blockhash")]
	#[serde(serialize_with = "serialize_h256_option")]
	#[serde(deserialize_with = "deserialize_h256_option")]
	pub block_hash: Option<H256>,

	#[serde(rename = "confirmations")]
	pub confirmations: Option<i32>,

	#[serde(rename = "blocktime")]
	pub block_time: Option<i32>,

	#[serde(rename = "vmstate")]
	pub vm_state: Option<VMState>,
}

impl PartialEq for Transaction {
	fn eq(&self, other: &Self) -> bool {
		self.hash == other.hash
	}
}
