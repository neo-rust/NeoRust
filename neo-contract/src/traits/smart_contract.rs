use crate::{iterator::NeoIterator, transaction_builder::TransactionBuilder};
use async_trait::async_trait;
use neo_types::{
	call_flags::CallFlags,
	contract_error::ContractError,
	contract_manifest::ContractManifest,
	contract_parameter::ContractParameter,
	invocation_result::InvocationResult,
	op_code::OpCode,
	script_hash::{ScriptHash, ScriptHashExtension},
	signers::signer::Signer,
	stack_item::StackItem,
	Bytes,
};
use num_bigint::BigInt;
use primitive_types::H160;
use rustc_serialize::hex::ToHex;
use std::sync::Arc;
use neo_builder::script::script_builder::ScriptBuilder;
use neo_builder::transaction::call_flags::CallFlags;
use neo_builder::transaction::signers::signer::Signer;
use neo_builder::transaction::transaction_builder::TransactionBuilder;

#[async_trait]
pub trait SmartContractTrait: Send + Sync {
	const DEFAULT_ITERATOR_COUNT: usize = 100;

	async fn name(&self) -> String {
		self.get_manifest().await.name.clone().unwrap()
	}
	fn set_name(&mut self, name: String) {
		panic!("Cannot set name for NNS")
	}

	fn script_hash(&self) -> H160;

	fn set_script_hash(&mut self, script_hash: H160) {
		panic!("Cannot set script hash for NNS")
	}

	async fn invoke_function(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<TransactionBuilder, ContractError> {
		let script = self.build_invoke_function_script(function, params).await.unwrap();
		let mut builder = TransactionBuilder::new();
		builder.set_script(script);
		Ok(builder)
	}

	async fn build_invoke_function_script(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<Bytes, ContractError> {
		if function.is_empty() {
			return Err(ContractError::InvalidNeoName("Function name cannot be empty".to_string()))
		}

		let script = ScriptBuilder::new()
			.contract_call(&self.script_hash(), function, params.as_slice(), CallFlags::None)
			.unwrap()
			.to_bytes();

		Ok(script)
	}

	async fn call_function_returning_string(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<String, ContractError> {
		let output = self.call_invoke_function(function, params, vec![]).await.unwrap();
		self.throw_if_fault_state(&output).unwrap();

		let item = output.stack[0].clone();
		match item.as_string() {
			Some(s) => Ok(s),
			None => Err(ContractError::UnexpectedReturnType("String".to_string())),
		}
	}

	async fn call_function_returning_int(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<i32, ContractError> {
		let output = self.call_invoke_function(function, params, vec![]).await.unwrap();
		self.throw_if_fault_state(&output).unwrap();

		let item = output.stack[0].clone();
		match item.as_int() {
			Some(i) => Ok(i as i32),
			None => Err(ContractError::UnexpectedReturnType("Int".to_string())),
		}
	}

	async fn call_function_returning_bool(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<bool, ContractError> {
		let output = self.call_invoke_function(function, params, vec![]).await.unwrap();
		self.throw_if_fault_state(&output).unwrap();

		let item = output.stack[0].clone();
		match item.as_bool() {
			Some(b) => Ok(b),
			None => Err(ContractError::UnexpectedReturnType("Bool".to_string())),
		}
		// .ok_or_else(|| ContractError::UnexpectedReturnType("Bool".to_string()))
	}

	// Other methods

	async fn call_invoke_function(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
		signers: Vec<Signer>,
	) -> Result<InvocationResult, ContractError> {
		if function.is_empty() {
			return Err(ContractError::from(ContractError::InvalidNeoName(
				"Function cannot be empty".to_string(),
			)))
		}

		let req = {
			let binding = NEO_INSTANCE.read().unwrap();
			let res = binding
				.invoke_function(&self.script_hash().clone(), function.into(), params, signers)
				.clone();
			res
		};
		req.request().await
	}

	fn throw_if_fault_state(&self, output: &InvocationResult) -> Result<(), ContractError> {
		if output.has_state_fault() {
			Err(ContractError::UnexpectedReturnType(output.exception.clone().unwrap()))
		} else {
			Ok(())
		}
	}

	// Other methods like `call_function_returning_xxx`, iterators, etc.
	async fn call_function_returning_script_hash(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<H160, ContractError> {
		let output = self.call_invoke_function(function, params, vec![]).await.unwrap();
		self.throw_if_fault_state(&output).unwrap();

		let item = &output.stack[0];
		item.as_bytes()
			.as_deref()
			.map(|b| ScriptHash::from_script(b))
			.ok_or_else(|| ContractError::UnexpectedReturnType("Script hash".to_string()))
	}

	async fn call_function_returning_iterator<U>(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
		mapper: Arc<dyn Fn(StackItem) -> U + Send + Sync>,
	) -> NeoIterator<U>
	where
		U: Send + Sync, // Adding this bound if necessary
	{
		let output = self.call_invoke_function(function, params, vec![]).await.unwrap();
		self.throw_if_fault_state(&output).unwrap();

		let item = &output.stack[0];
		let StackItem::InteropInterface { id, interface } = item else { panic!("") };

		let session_id = output
			.session_id
			.ok_or(ContractError::InvalidNeoNameServiceRoot("No session ID".to_string()))
			.unwrap();

		NeoIterator::new(session_id, id.clone(), mapper)
	}

	async fn call_function_and_unwrap_iterator<U>(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
		max_items: usize,
		mapper: impl Fn(StackItem) -> U + Send,
	) -> Result<Vec<U>, ContractError> {
		let script = ScriptBuilder::build_contract_call_and_unwrap_iterator(
			&self.script_hash(),
			function,
			&params,
			255, //TODO
			CallFlags::All,
		)
		.unwrap();

		let output = { NEO_INSTANCE.read().unwrap().invoke_script(script.to_hex(), vec![]) };

		let output = output.request().await.unwrap();

		self.throw_if_fault_state(&output).unwrap();

		let items = output.stack[0].as_array().unwrap().into_iter().map(mapper).collect();

		Ok(items)
	}

	fn calc_native_contract_hash(contract_name: &str) -> Result<H160, ContractError> {
		Self::calc_contract_hash(H160::zero(), 0, contract_name)
	}

	fn calc_contract_hash(
		sender: H160,
		nef_checksum: u32,
		contract_name: &str,
	) -> Result<H160, ContractError> {
		let mut script = ScriptBuilder::new();
		script.op_code(&[OpCode::Abort]);
		script.push_data(sender.to_vec()).unwrap();
		script.push_integer(BigInt::from(nef_checksum)).unwrap();
		script.push_data(contract_name.as_bytes().to_vec()).unwrap();

		Ok(H160::from_slice(&script.to_bytes()))
	}

	async fn get_manifest(&self) -> ContractManifest {
		let req = { NEO_INSTANCE.read().unwrap().get_contract_state(self.script_hash()).clone() };

		req.request().await.unwrap().manifest.clone()
	}
}