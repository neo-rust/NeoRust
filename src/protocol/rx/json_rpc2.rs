use crate::{
	neo_error::NeoError,
	protocol::{
		core::{
			block_index::BlockIndexPolling, neo_trait::NeoTrait,
			responses::neo_response_aliases::NeoGetBlock,
		},
		http_service::HttpService,
		neo_rust::NeoRust,
	},
};
use futures::{Stream, StreamExt, TryStreamExt};
use std::time::Duration;
use tokio::{runtime::Handle, time::interval};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonRpc2 {
	executor_service: Handle,
}

impl JsonRpc2 {
	pub fn new(executor_service: Handle) -> Self {
		Self { executor_service }
	}

	pub async fn block_index_publisher(
		&self,
		polling_interval: i32,
	) -> impl Stream<Item = Result<i32, NeoError>> {
		BlockIndexPolling::block_index_publisher(&self.executor_service, polling_interval, 0)
	}

	pub async fn block_publisher(
		&self,
		full_transaction_objects: bool,
		polling_interval: i32,
	) -> impl Stream<Item = Result<NeoGetBlock, NeoError>> {
		self.block_index_publisher(polling_interval).and_then(|index| {
			NeoRust::<HttpService>::instance()
				.get_block(index, full_transaction_objects)
				.execute(&mut self.executor_service)
		})
	}

	pub async fn replay_blocks_publisher(
		&mut self,
		start_block: i32,
		end_block: i32,
		full_transaction_objects: bool,
		ascending: bool,
	) -> impl Stream<Item = Result<NeoGetBlock, NeoError>> {
		let mut blocks = (start_block..=end_block).collect::<Vec<_>>();
		if !ascending {
			blocks.reverse();
		}

		futures::stream::iter(blocks).and_then(|block| {
			NeoRust::<HttpService>::instance()
				.get_block(block, full_transaction_objects)
				.execute(&mut self.executor_service)
		})
	}

	pub async fn catch_up_to_latest_block_publisher(
		&self,
		start_block: i32,
		full_transaction_objects: bool,
		on_caught_up_publisher: impl Stream<Item = Result<NeoGetBlock, NeoError>>,
	) -> impl Stream<Item = Result<NeoGetBlock, NeoError>> {
		let latest_block = self.latest_block_index_publisher().await.unwrap();

		if start_block >= latest_block {
			Box::pin(on_caught_up_publisher)
		} else {
			let replay_stream =
				self.replay_blocks_publisher(start_block, latest_block, full_transaction_objects);

			let new_publisher = self.catch_up_to_latest_block_publisher(
				latest_block + 1,
				full_transaction_objects,
				on_caught_up_publisher,
			);

			replay_stream.chain(new_publisher)
		}
	}
	pub async fn catch_up_to_latest_and_subscribe_to_new_blocks_publisher(
		&self,
		start_block: i32,
		full_transaction_objects: bool,
		polling_interval: i32,
	) -> impl Stream<Item = Result<NeoGetBlock, NeoError>> {
		self.catch_up_to_latest_block_publisher(
			start_block,
			full_transaction_objects,
			self.block_publisher(full_transaction_objects, polling_interval),
		)
	}

	pub async fn latest_block_index_publisher(&self) -> Result<i32, NeoError> {
		NeoRust::<HttpService>::instance()
			.get_block_count()
			.execute(&mut self.executor_service)
			.unwrap()
			.get_result()
			- 1
	}
}
