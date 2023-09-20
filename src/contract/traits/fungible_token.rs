use crate::{
	contract::{contract_error::ContractError, nns_name::NNSName, traits::token::TokenTrait},
	transaction::{account_signer::AccountSigner, transaction_builder::TransactionBuilder},
	types::{contract_parameter::ContractParameter, Bytes},
	wallet::{account::Account, wallet::Wallet},
};
use async_trait::async_trait;
use primitive_types::H160;

#[async_trait]
pub trait FungibleTokenTrait: TokenTrait {
	const BALANCE_OF: &'static str = "balanceOf";
	const TRANSFER: &'static str = "transfer";

	async fn get_balance_of(&self, account: &Account) -> Result<i32, ContractError> {
		self.get_balance_of_hash160(account.get_script_hash()?).await
	}

	async fn get_balance_of_hash160(&self, script_hash: H160) -> Result<i32, ContractError> {
		self.call_function_returning_int(FungibleTokenTrait::BALANCE_OF, vec![script_hash.into()])
			.await
	}

	async fn get_total_balance(&self, wallet: &Wallet) -> Result<i32, ContractError> {
		let mut sum = 0;
		for (_, account) in &wallet.accounts {
			sum += self.get_balance_of(account).await?;
		}
		Ok(sum)
	}

	fn transfer_from_account(
		&self,
		from: &Account,
		to: H160,
		amount: i32,
		data: Option<ContractParameter>,
	) -> Result<TransactionBuilder, ContractError> {
		self.transfer_from_hash160(from.get_script_hash()?, to, amount, data)
			.map(|b| b.signers(vec![AccountSigner::called_by_entry(from)]))
	}

	fn transfer_from_hash160(
		&self,
		from: H160,
		to: H160,
		amount: i32,
		data: Option<ContractParameter>,
	) -> Result<TransactionBuilder, ContractError> {
		if amount < 0 {
			return Err(ContractError::InvalidArgError(
				"The amount must be greater than or equal to 0.".to_string(),
			))
		}

		let transfer_script = self.build_transfer_script(from, to, amount, data)?;
		Ok(TransactionBuilder::new().script(transfer_script))
	}

	fn build_transfer_script(
		&self,
		from: H160,
		to: H160,
		amount: i32,
		data: Option<ContractParameter>,
	) -> Result<Bytes, ContractError> {
		self.build_invoke_function_script(
			FungibleTokenTrait::TRANSFER,
			vec![from.into(), to.into(), amount.into(), data],
		)
	}

	// MARK: Transfer using NNS

	async fn transfer_from_account_to_nns(
		&self,
		from: &Account,
		to: &NNSName,
		amount: i32,
		data: Option<ContractParameter>,
	) -> Result<TransactionBuilder, ContractError> {
		self.transfer_from_hash160_to_nns(from.get_script_hash()?, to, amount, data)
			.await
			.map(|b| b.signers(vec![AccountSigner::called_by_entry(from)]))
	}

	async fn transfer_from_hash160_to_nns(
		&self,
		from: H160,
		to: &NNSName,
		amount: i32,
		data: Option<ContractParameter>,
	) -> Result<TransactionBuilder, ContractError> {
		let script_hash = self.resolve_nns_text_record(to).await?;
		self.transfer_from_hash160(from, script_hash, amount, data)
	}
}