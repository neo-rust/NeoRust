use getset::{Getters, Setters};
use neo_codec::Decoder;
use num_bigint::BigInt;
use p256::{ecdsa::Signature, pkcs8::der::Encode, PublicKey};
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, vec};

use crate::{error::TypeError, op_code::OpCode, Bytes};
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Getters, Setters)]
pub struct VerificationScript {
	#[getset(get = "pub", set = "pub")]
	script: Bytes,
}

impl VerificationScript {
	pub fn new() -> Self {
		Self { script: Bytes::new() }
	}

	pub fn from(script: Bytes) -> Self {
		Self { script: script.to_vec() }
	}

	pub fn from_public_key(public_key: &PublicKey) -> Self {
		let mut builder = ScriptBuilder::new();
		builder
			.push_data(public_key.to_encoded_point(false).as_bytes().to_vec())
			.unwrap()
			.op_code(&vec![OpCode::Syscall])
			.push_data(InteropService::SystemCryptoCheckSig.hash().into_bytes())
			.unwrap();
		Self::from(builder.to_bytes())
	}

	pub fn from_MultiSig(public_keys: &[PublicKey], threshold: u8) -> Self {
		// Build multi-sig script
		let mut builder = ScriptBuilder::new();
		builder
			.push_integer(BigInt::from(threshold))
			.expect("Threshold must be between 1 and 16");
		for key in public_keys {
			builder.push_data(key.to_vec()).unwrap();
		}
		builder
			.push_integer(BigInt::from(public_keys.len()))
			.unwrap()
			.op_code(vec![OpCode::Syscall].as_slice())
			.push_data(InteropService::SystemCryptoCheckMultiSig.hash().into_bytes())
			.unwrap();
		Self::from(builder.to_bytes())
	}

	pub fn is_single_sig(&self) -> bool {
		self.script.len() == 35
			&& self.script[0] == OpCode::PushData1 as u8
			&& self.script[34] == OpCode::Syscall as u8
	}

	pub fn is_MultiSig(&self) -> bool {
		if self.script.len() < 37 {
			return false
		}

		let mut reader = Decoder::new(&self.script);

		let n = reader.by_ref().read_var_int().unwrap();
		if !(1..16).contains(&n) {
			return false
		}

		let mut m = 0;
		while reader.by_ref().read_u8() == OpCode::PushData1 as u8 {
			let len = reader.by_ref().read_u8();
			if len != 33 {
				return false
			}
			let _ = reader.by_ref().skip(33);
			m += 1;
		}

		if !(m >= n && m <= 16) {
			return false
		}

		// additional checks
		let service_bytes = &self.script[self.script.len() - 4..];
		if service_bytes != &InteropService::SystemCryptoCheckMultiSig.hash().into_bytes() {
			return false
		}

		if m != reader.by_ref().read_var_int().unwrap() {
			return false
		}

		if reader.by_ref().read_u8() != OpCode::Syscall as u8 {
			return false
		}

		true
	}

	// other methods
	pub fn hash(&self) -> H160 {
		H160::from_slice(&self.script)
	}

	pub fn get_signatures(&self) -> Vec<Signature> {
		let mut reader = Decoder::new(&self.script);
		let mut signatures = vec![];

		while reader.by_ref().read_u8() == OpCode::PushData1 as u8 {
			let len = reader.by_ref().read_u8();
			let sig =
				Signature::from_der(&reader.by_ref().read_bytes(len as usize).unwrap()).unwrap();
			signatures.push(sig);
		}

		signatures
	}

	pub fn get_public_keys(&self) -> Result<Vec<PublicKey>, TypeError> {
		if self.is_single_sig() {
			let mut reader = Decoder::new(&self.script);
			reader.by_ref().read_u8(); // skip pushdata1
			reader.by_ref().read_u8(); // skip length

			let mut point = [0; 33];
			point.copy_from_slice(&reader.by_ref().read_bytes(33).unwrap());

			let key = PublicKey::from_sec1_bytes(&point).unwrap();
			return Ok(vec![key])
		}

		if self.is_MultiSig() {
			let mut reader = Decoder::new(&self.script);
			reader.by_ref().read_var_int().unwrap(); // skip threshold

			let mut keys = vec![];
			while reader.by_ref().read_u8() == OpCode::PushData1 as u8 {
				reader.by_ref().read_u8(); // skip length
				let mut point = [0; 33];
				point.copy_from_slice(&reader.by_ref().read_bytes(33).unwrap());
				keys.push(PublicKey::from_sec1_bytes(&point).unwrap());
			}

			return Ok(keys)
		}

		Err(TypeError::InvalidScript("Invalid verification script".to_string()))
	}

	pub fn get_signing_threshold(&self) -> Result<usize, TypeError> {
		if self.is_single_sig() {
			Ok(1)
		} else if self.is_MultiSig() {
			let reader = &mut Decoder::new(&self.script);
			Ok(reader.by_ref().read_var_int()? as usize)
		} else {
			Err(TypeError::InvalidScript("Invalid verification script".to_string()))
		}
	}
	pub fn get_nr_of_accounts(&self) -> Result<usize, TypeError> {
		match self.get_public_keys() {
			Ok(keys) => Ok(keys.len()),
			Err(e) => Err(e),
		}
	}
}
