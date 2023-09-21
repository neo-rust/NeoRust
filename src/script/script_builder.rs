use crate::{
	crypto::key_pair::KeyPair,
	neo_error::NeoError,
	script::{interop_service::InteropService, op_code::OpCode},
	types::{
		call_flags::CallFlags,
		contract_parameter::{ContractParameter, ParameterValue},
		contract_parameter_type::ContractParameterType,
		Bytes, PublicKey,
	},
};
use p256::{elliptic_curve::sec1::ToEncodedPoint, pkcs8::der::Encode};
use primitive_types::H160;
use std::{collections::HashMap, error::Error};
use tokio::io::AsyncWriteExt;

#[derive(Debug, PartialEq, Eq, Hash, CopyGetters, Setters)]
#[getset(get_copy, set)]
#[derive(educe::Educe)]
// note `new` below: generate `new()` that calls Default
#[educe(Default(new))]
pub struct ScriptBuilder {
	pub(crate) script: Vec<u8>,
}

impl ScriptBuilder {
	// pub fn new() -> Self {
	// 	Self { writer: BinaryWriter::new() }
	// }

	pub async fn op_code(&mut self, op_codes: &[OpCode]) -> &mut Self {
		for opcode in op_codes {
			self.script.write_u8(opcode.opcode()).await.expect("Failed to write opcode");
		}
		self
	}

	pub async fn op_code_with_arg(&mut self, opcode: OpCode, argument: Bytes) -> &mut Self {
		self.script.write_u8(opcode.opcode()).await.expect("");
		let _ = self.script.write(&argument);
		self
	}

	pub async fn contract_call(
		&mut self,
		hash160: &H160,
		method: &str,
		params: &[ContractParameter],
		call_flags: CallFlags,
	) -> Result<&mut Self, NeoError> {
		if params.is_empty() {
			self.op_code(&[OpCode::NewArray]).await;
		} else {
			self.push_params(params);
		}

		self.push_integer(call_flags.bits())
			.unwrap()
			.push_data(method.as_bytes().to_vec())
			.push_data(hash160.to_bytes())
			.sys_call(InteropService::SystemContractCall)
	}

	pub fn sys_call(&mut self, operation: InteropService) -> &mut Self {
		self.op_code(&[OpCode::Syscall])
			.push_data(operation.to_hash().as_bytes())
			.unwrap()
	}

	pub fn push_params(&mut self, params: &[ContractParameter]) -> &mut Self {
		for param in params {
			self.push_param(param).unwrap();
		}

		self.push_integer(params.len() as i64).unwrap().op_code(&[OpCode::Pack])
	}

	pub async fn push_param(&mut self, param: &ContractParameter) -> Result<&mut Self, NeoError> {
		if param.get_type() == ContractParameterType::Any {
			self.op_code(&[OpCode::PushNull]).await;
		}
		match &param.value.unwrap() {
			ParameterValue::Boolean(b) => self.push_bool(*b),
			ParameterValue::Integer(i) => self.push_integer(i.clone()).unwrap(),
			ParameterValue::ByteArray(b)
			| ParameterValue::Signature(b)
			| ParameterValue::PublicKey(b) => self.push_data(b.as_bytes().to_vec()),
			ParameterValue::Hash160(h) => self.push_data(h.as_bytes().to_vec()),
			ParameterValue::Hash256(h) => self.push_data(h.as_bytes().to_vec()),
			ParameterValue::String(s) => self.push_data(s.as_bytes().to_vec()),
			ParameterValue::Array(arr) => self.push_array(arr).unwrap(),
			ParameterValue::Map(map) => {
				// Create an empty HashMap to hold your ContractParameter key-value pairs
				let mut map: HashMap<ContractParameter, ContractParameter> = HashMap::new();

				// Iterate over pairs of elements in the vector
				// (assuming the vector has an even number of elements)
				for i in (0..map.len()).step_by(2) {
					let key = ContractParameter::from_json_value(map[i].clone());
					let value = ContractParameter::from_json_value(map[i + 1].clone());

					// Insert the key-value pair into the HashMap
					map.insert(key, value);
				}

				self.push_map(&map).unwrap()
			},
			_ => return Err(Error::IllegalArgument("Unsupported parameter type".to_string())),
		}
		.await;

		Ok(self)
	}

	// Additional push_* methods

	pub async fn push_integer(&mut self, n: i64) -> Result<&mut Self, NeoError> {
		if n == -1 {
			self.op_code(&[OpCode::PushM1]).await;
		} else if 0 <= n && n <= 16 {
			self.op_code(&[OpCode::from_u8(OpCode::Push0.into() + n as u8).unwrap()]).await;
		} else {
			let mut bytes = n.to_be_bytes();
			match self.script.len() {
				1 => self.op_code_with_arg(OpCode::PushInt8, bytes.to_vec().unwrap()),
				2 => self.op_code_with_arg(OpCode::PushInt16, bytes.to_vec().unwrap()),
				4 => self.op_code_with_arg(OpCode::PushInt32, bytes.to_vec().unwrap()),
				8 => self.op_code_with_arg(OpCode::PushInt64, bytes.to_vec().unwrap()),
				16 => self.op_code_with_arg(OpCode::PushInt128, bytes.to_vec().unwrap()),
				32 => self.op_code_with_arg(OpCode::PushInt256, bytes.to_vec().unwrap()),
				_ => return Err(Error::NumericOverflow),
			}
			.await;
		}
		Ok(self)
	}

	fn pad_number(&self, n: i128, size: usize) -> Bytes {
		let mut bytes = n.to_vec().unwrap(); // .to_signed_bytes();
		if self.script.len() == size {
			return bytes
		}
		let pad_byte = if n.is_negative() { 0xff } else { 0 };
		if n.is_negative() {
			let mut padding =
				Bytes::from_iter(std::iter::repeat(pad_byte).take(size - &self.script.len()));
			padding.append(&mut bytes);
			padding
		} else {
			let mut result = bytes;
			result.resize(size, pad_byte);
			result
		}
	}

	// Push data handling

	pub async fn push_data(&mut self, data: Bytes) -> Result<&mut Self, NeoError> {
		match data.len() {
			0..=75 => {
				self.op_code(&[OpCode::PushData1]).await;
				self.script.write_u8(data.len() as u8).await.expect("Failed to write data");
				let _ = self.script.write(&data);
			},
			76..=0xff => {
				self.op_code(&[OpCode::PushData2]).await;
				self.script.write_u16(data.len() as u16).await.expect("Failed to write data");
				let _ = self.script.write(&data);
			},
			0x100..=0xffff => {
				self.op_code(&[OpCode::PushData4]).await;
				self.script.write_u32(data.len() as u32).await.expect("Failed to write data");
				let _ = self.script.write(&data);
			},
			_ => return Err(NeoError::IllegalArgument("Data too long".to_string())),
		}
		Ok(self)
	}

	pub async fn push_bool(&mut self, b: bool) -> &mut Self {
		if b { self.op_code(&[OpCode::PushTrue]) } else { self.op_code(&[OpCode::PushFalse]) }
			.await;
		self
	}

	pub async fn push_array(&mut self, arr: &[ContractParameter]) -> Result<&mut Self, NeoError> {
		if arr.is_empty() {
			self.op_code(&[OpCode::NewArray])
		} else {
			let arrr = arr
				.iter()
				.map(|v| {
					let vv: ContractParameter = v.clone().into();
					vv
				})
				.collect();
			self.push_params(&Some(arrr)).unwrap();
		}
		.await;
		Ok(self)
	}

	pub fn push_map(
		&mut self,
		map: &HashMap<ContractParameter, ContractParameter>,
	) -> Result<&mut Self, NeoError> {
		for (k, v) in map {
			let kk: ContractParameter = k.clone().into();
			let vv: ContractParameter = v.clone().into();
			self.push_param(&vv).unwrap();
			self.push_param(&kk).unwrap();
		}

		Ok(self.push_integer(map.len() as i64).unwrap().op_code(&[OpCode::PackMap]))
	}

	// Additional helper methods

	pub fn pack(&mut self) -> &mut Self {
		self.op_code(&[OpCode::Pack])
	}

	pub fn to_bytes(&self) -> Bytes {
		self.script.into_bytes()
	}

	pub fn build_verification_script(pub_key: &PublicKey) -> Bytes {
		let mut sb = ScriptBuilder::new();
		sb.push_data(pub_key.to_encoded_point(false).as_bytes().to_vec())
			.unwrap()
			.sys_call(InteropService::SystemCryptoCheckSig);
		sb.to_bytes()
	}

	pub fn build_multisig_script(pubkeys: &[KeyPair], threshold: u8) -> Result<Bytes, NeoError> {
		let mut sb = ScriptBuilder::new();
		sb.push_int(threshold as i64).unwrap();
		for pk in pubkeys
			.iter()
			.sorted_by(|a, b| a.to_encoded_point(true).cmp(&b.to_encoded_point(true)))
		{
			sb.push_data(pk.to_encoded_point(true)).unwrap();
		}
		sb.push_int(pubkeys.len() as i64).unwrap();
		sb.sys_call(InteropService::SystemCryptoCheckMultisig);
		Ok(sb.to_bytes())
	}

	pub fn build_contract_script(
		sender: &H160,
		nef_checksum: u32,
		name: &str,
	) -> Result<Bytes, NeoError> {
		let mut sb = ScriptBuilder::new();
		sb.op_code(&[OpCode::Abort])
			.unwrap()
			.push_data(sender.to_array().unwrap().as_slice())
			.unwrap()
			.push_int(nef_checksum as i64)
			.unwrap()
			.push_data(name.as_bytes())
			.unwrap();
		Ok(sb.to_bytes())
	}
	pub async fn build_contract_call_and_unwrap_iterator(
		contract_hash: &H160,
		method: &str,
		params: &[ContractParameter],
		max_items: u32,
		call_flags: CallFlags,
	) -> Result<Bytes, NeoError> {
		let mut sb = Self::new();
		sb.push_integer(max_items as i64).unwrap();

		sb.contract_call(contract_hash, method, params, call_flags).unwrap();

		sb.op_code(&[OpCode::NewArray]).await;

		let cycle_start = sb.script.len();
		sb.op_code(&[OpCode::Over]).await;
		sb.sys_call(InteropService::SystemIteratorNext).unwrap();

		let jmp_if_not = sb.script.len();
		sb.op_code_with_arg(OpCode::JmpIf, vec![0]).unwrap();

		sb.op_code(&[OpCode::Dup, OpCode::Push2, OpCode::Pick])
			.sys_call(InteropService::SystemIteratorValue)
			.op_code(&[
				OpCode::Append,
				OpCode::Dup,
				OpCode::Size,
				OpCode::Push3,
				OpCode::Pick,
				OpCode::Ge,
			])
			.unwrap();

		let jmp_if_max = sb.script.len();
		sb.op_code_arg(OpCode::JmpIf, &[0]).unwrap();

		let jmp_offset = sb.script.len();
		let jmp_bytes = (cycle_start - jmp_offset) as i8;
		sb.op_code_arg(OpCode::Jmp, &[jmp_bytes]).unwrap();

		let load_result = sb.script.len();
		sb.op_code(&[OpCode::Nip, OpCode::Nip]).unwrap();

		let mut script = sb.to_bytes();
		let jmp_not_bytes = (load_result - jmp_if_not) as i8;
		script[jmp_if_not + 1] = jmp_not_bytes as u8;

		let jmp_max_bytes = (load_result - jmp_if_max) as i8;
		script[jmp_if_max + 1] = jmp_max_bytes as u8;

		Ok(script)
	}

	// Other static helper methods
}
