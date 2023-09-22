use crate::{
	constant::NeoConstants,
	neo_error::NeoError,
	protocol::{
		core::{neo_trait::NeoTrait, responses::transaction_attribute::TransactionAttribute},
		neo_rust::NeoRust,
	},
	transaction::{
		account_signer::AccountSigner,
		contract_signer::ContractSigner,
		serializable_transaction::SerializableTransaction,
		signer::{Signer, SignerType},
		transaction_error::TransactionError,
		witness::Witness,
	},
	types::{
		contract_parameter::ContractParameter, Bytes, H160Externsion, PublicKey, PublicKeyExtension,
	},
};
use bincode::Options;
use hex_literal::hex;
use primitive_types::H160;
use rustc_serialize::hex::ToHex;
use serde::Serialize;
use std::{
	collections::HashSet,
	fmt::Debug,
	hash::{Hash, Hasher},
	str::FromStr,
};

#[derive(Getters, Setters, MutGetters, CopyGetters, Default)]
pub struct TransactionBuilder {
	version: u8,
	nonce: u32,
	valid_until_block: Option<u32>,
	// setter and getter
	#[getset(get = "pub", set = "pub")]
	signers: Vec<Signer>,
	additional_network_fee: u64,
	additional_system_fee: u64,
	attributes: Vec<TransactionAttribute>,
	script: Option<Bytes>,
	fee_consumer: Option<Box<dyn Fn(u64, u64)>>,
	fee_error: Option<TransactionError>,
}

impl Debug for TransactionBuilder {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("TransactionBuilder")
			.field("version", &self.version)
			.field("nonce", &self.nonce)
			.field("valid_until_block", &self.valid_until_block)
			.field("signers", &self.signers)
			.field("additional_network_fee", &self.additional_network_fee)
			.field("additional_system_fee", &self.additional_system_fee)
			.field("attributes", &self.attributes)
			.field("script", &self.script)
			// .field("fee_consumer", &self.fee_consumer)
			.field("fee_error", &self.fee_error)
			.finish()
	}
}

impl Clone for TransactionBuilder {
	fn clone(&self) -> Self {
		Self {
			version: self.version,
			nonce: self.nonce,
			valid_until_block: self.valid_until_block,
			signers: self.signers.clone(),
			additional_network_fee: self.additional_network_fee,
			additional_system_fee: self.additional_system_fee,
			attributes: self.attributes.clone(),
			script: self.script.clone(),
			// fee_consumer: self.fee_consumer.clone(),
			fee_consumer: None,
			fee_error: self.fee_error.clone(),
		}
	}
}

impl Eq for TransactionBuilder {}

impl PartialEq for TransactionBuilder {
	fn eq(&self, other: &Self) -> bool {
		self.version == other.version
			&& self.nonce == other.nonce
			&& self.valid_until_block == other.valid_until_block
			&& self.signers == other.signers
			&& self.additional_network_fee == other.additional_network_fee
			&& self.additional_system_fee == other.additional_system_fee
			&& self.attributes == other.attributes
			&& self.script == other.script
	}
}

impl Hash for TransactionBuilder {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.version.hash(state);
		self.nonce.hash(state);
		self.valid_until_block.hash(state);
		self.signers.hash(state);
		self.additional_network_fee.hash(state);
		self.additional_system_fee.hash(state);
		self.attributes.hash(state);
		self.script.hash(state);
	}
}

impl TransactionBuilder {
	pub const GAS_TOKEN_HASH: [u8; 20] = hex!("d2a4cff31913016155e38e474a2c06d08be276cf");
	pub const BALANCE_OF_FUNCTION: &'static str = "balanceOf";
	pub const DUMMY_PUB_KEY: &'static str =
		"02ec143f00b88524caf36a0121c2de09eef0519ddbe1c710a00f0e2663201ee4c0";

	// Constructor
	pub fn new() -> Self {
		Self {
			version: 0,
			nonce: 0,
			valid_until_block: None,
			signers: Vec::new(),
			additional_network_fee: 0,
			additional_system_fee: 0,
			attributes: Vec::new(),
			script: None,
			fee_consumer: None,
			fee_error: None,
		}
	}

	// Configuration

	pub fn version(&mut self, version: u8) -> &mut Self {
		self.version = version;
		self
	}

	pub fn nonce(&mut self, nonce: u32) -> Result<&mut Self, TransactionError> {
		// Validate
		if nonce >= u32::MAX {
			return Err(TransactionError::InvalidNonce)
		}

		self.nonce = nonce;
		Ok(self)
	}

	// Other methods

	// Set valid until block
	pub fn valid_until_block(&mut self, block: u32) -> Result<&mut Self, TransactionError> {
		if block == 0 {
			return Err(TransactionError::InvalidBlock)
		}

		self.valid_until_block = Some(block);
		Ok(self)
	}

	// Set script
	pub fn set_script(&mut self, script: Bytes) -> &mut Self {
		self.script = Some(script);
		self
	}

	// Get unsigned transaction
	pub async fn get_unsigned_tx(&mut self) -> Result<SerializableTransaction, TransactionError> {
		// Validate configuration
		if self.signers.is_empty() {
			return Err(TransactionError::NoSigners)
		}

		if self.script.is_none() {
			return Err(TransactionError::NoScript)
		}
		let len = self.signers.len();
		self.signers.dedup();

		// Validate no duplicate signers
		if len != self.signers.len() {
			return Err(TransactionError::DuplicateSigner)
		}

		// Check signer limits
		if self.signers.len() > NeoConstants::MAX_SIGNER_SUBITEMS as usize {
			return Err(TransactionError::TooManySigners)
		}

		// Validate script
		if let Some(script) = &self.script {
			if script.is_empty() {
				return Err(TransactionError::EmptyScript)
			}
		} else {
			return Err(TransactionError::NoScript)
		}

		let mut tx = SerializableTransaction::new(
			self.version,
			self.nonce,
			self.valid_until_block.unwrap(),
			self.clone().signers,
			0,
			0,
			self.clone().attributes,
			self.clone().script.unwrap(),
			vec![],
		);
		// Get fees
		let system_fee = self.get_system_fee().await.unwrap();
		let network_fee = self.get_network_fee(&tx).await.unwrap();

		// Check sender balance if needed
		if let Some(fee_consumer) = &self.fee_consumer {
			let sender_balance = self.get_sender_balance().await.unwrap();
			if network_fee + system_fee > sender_balance {
				fee_consumer(network_fee + system_fee, sender_balance);
			}
		}

		tx.set_network_fee(network_fee as i64);
		tx.set_system_fee(system_fee as i64);
		// Build transaction
		Ok(tx)
	}

	async fn get_system_fee(&self) -> Result<u64, TransactionError> {
		let script = self.script.as_ref().unwrap();

		let response = NeoRust::instance()
			.invoke_script(script.to_hex(), vec![self.signers[0].clone()])
			.request()
			.await
			.unwrap();
		Ok(u64::from_str(response.gas_consumed.as_str()).unwrap()) // example
	}

	async fn get_network_fee(
		&mut self,
		tx: &SerializableTransaction,
	) -> Result<u64, TransactionError> {
		let fee = NeoRust::instance()
			.calculate_network_fee(tx.serialize().to_hex())
			.request()
			.await
			.unwrap()
			.network_fee;
		Ok(fee)
	}

	// Get sender balance
	async fn get_sender_balance(&self) -> Result<u64, TransactionError> {
		// Call network
		let sender = &self.signers[0];

		if Self::is_account_signer(sender) {
			let balance = NeoRust::instance()
				.invoke_function(
					&H160::from(Self::GAS_TOKEN_HASH),
					Self::BALANCE_OF_FUNCTION.to_string(),
					vec![ContractParameter::hash160(sender.get_signer_hash())],
					vec![],
				)
				.request()
				.await
				.unwrap()
				.stack[0]
				.clone();
			return Ok(balance.as_int().unwrap() as u64)
		}
		Err(TransactionError::InvalidSender)
	}

	fn is_account_signer(signer: &Signer) -> bool {
		// let sig = <T as Signer>::SignerType;
		if signer.get_type() == SignerType::Account {
			return true
		}
		return false
	}

	// Sign transaction
	pub async fn sign(&mut self) -> Result<SerializableTransaction, NeoError> {
		let mut transaction = self.get_unsigned_transaction().await.unwrap();

		for signer in &mut transaction.signers {
			if Self::is_account_signer(signer) {
				let account_signer: AccountSigner = signer.into();
				let acc = &account_signer.account;
				if acc.is_multi_sig() {
					return Err(NeoError::IllegalState(
						"Transactions with multi-sig signers cannot be signed automatically."
							.to_string(),
					))
				}

				let key_pair = acc.key_pair.as_ref().ok_or_else(|| {
                  NeoError::InvalidConfiguration(
                      "Cannot create transaction signature because account does not hold a private key."
                          .to_string(),
                  )
              }).unwrap();

				let tx_bytes = transaction.get_hash_data().await.unwrap();
				transaction.add_witness(Witness::create(tx_bytes, key_pair).await.unwrap());
			} else {
				let contract_signer: &mut ContractSigner = signer.into();
				transaction.add_witness(
					Witness::create_contract_witness(contract_signer.verify_params.clone())
						.unwrap(),
				);
			}
		}

		Ok(transaction)
	}

	// Inside TransactionBuilder impl

	pub async fn get_unsigned_transaction(
		&mut self,
	) -> Result<SerializableTransaction, TransactionError> {
		if self.script.is_none() {
			return Err(TransactionError::TransactionConfiguration(
				"Cannot build a transaction without a script.".to_string(),
			))
		}

		if self.valid_until_block.is_none() {
			let current_block_count =
				NeoRust::instance().get_block_count().request().await.unwrap();
			self.valid_until_block = Some(
				(current_block_count + NeoRust::instance().max_valid_until_block_increment() - 1)
					as u32,
			);
		}

		if self.signers.is_empty() {
			return Err(NeoError::IllegalState(
				"Cannot create a transaction without signers.".to_string(),
			)
			.into())
		}

		if self.is_high_priority() {
			let is_allowed = self.is_allowed_for_high_priority().await.unwrap();
			if !is_allowed {
				return Err(NeoError::IllegalState(
					"Only committee members can send high priority transactions.".to_string(),
				)
				.into())
			}
		}
		let mut transaction = SerializableTransaction::new(
			self.version,
			self.nonce,
			self.valid_until_block.unwrap(),
			self.signers.clone(),
			0,
			0,
			self.attributes.clone(),
			self.script.as_ref().unwrap().clone(),
			vec![],
		);

		let system_fee = self.get_system_fee().await.unwrap();
		let network_fee = self.get_network_fee(&transaction).await.unwrap();
		let fees = system_fee + network_fee;

		if let Some(fee_error) = &self.fee_error {
			if !self.can_send_cover_fees(fees).await.unwrap() {
				return Err(fee_error.clone())
			}
		} else if let Some(consumer) = &mut self.clone().fee_consumer {
			let gas_balance = self.get_sender_gas_balance().await.unwrap();
			consumer(fees, gas_balance);
		}
		transaction.set_network_fee(network_fee as i64);
		transaction.set_system_fee(system_fee as i64);
		Ok(transaction)
	}

	async fn is_allowed_for_high_priority(&self) -> Result<bool, NeoError> {
		let committee = NeoRust::instance()
			.get_committee()
			.request()
			.await?
			.into_iter()
			.map(|key| PublicKey::from_hex(&key))
			.map(|key| key.unwrap().to_address_h160())
			.collect::<HashSet<_>>();

		Ok(self
			.signers
			.iter()
			.map(|s| s.get_signer_hash())
			.any(|hash| committee.contains(&hash))
			|| self.signers_contain_multisig_with_committee_member(&committee))
	}

	fn signers_contain_multisig_with_committee_member(&self, committee: &HashSet<H160>) -> bool {
		for signer in &self.signers {
			if let Some(account_signer) = signer.as_account_signer() {
				if account_signer.is_multisig() {
					if let Some(script) = &account_signer.account().verification_script {
						for pubkey in script.get_public_keys().unwrap() {
							let hash = pubkey.to_address_h160();
							if committee.contains(&hash) {
								return true
							}
						}
					}
				}
			}
		}

		false
	}

	fn size(&self) -> usize {
		let mut size = 0;

		// Add fixed header sizes
		size += 1; // version
		size += 4; // nonce
		size += 4; // valid until block

		// Add signers
		for signer in &self.signers {
			size += signer.serialized_size();
		}

		// Add attributes
		for attr in &self.attributes {
			size += attr.serialized_size();
		}

		// Add script
		if let Some(script) = &self.script {
			size += script.len() + 1;
		}

		size
	}

	pub fn is_high_priority(&self) -> bool {
		self.attributes
			.iter()
			.any(|attr| matches!(attr, TransactionAttribute::HighPriority))
	}

	async fn can_send_cover_fees(&self, fees: u64) -> Result<bool, NeoError> {
		let balance = self.get_sender_gas_balance().await?;
		Ok(balance >= fees)
	}

	async fn get_sender_gas_balance(&self) -> Result<u64, NeoError> {
		let sender_hash = self.signers[0].get_signer_hash();
		let result = NeoRust::instance()
			.invoke_function(
				&H160::from(Self::GAS_TOKEN_HASH),
				Self::BALANCE_OF_FUNCTION.to_string(),
				vec![sender_hash.into()],
				vec![],
			)
			.request()
			.await?;

		Ok(result.stack[0].as_int().unwrap() as u64)
	}
}
