use std::sync::Arc;
use codec::Codec;
use sp_blockchain::HeaderBackend;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_runtime::{generic::BlockId, traits::{Block as BlockT, MaybeDisplay}};
use sp_api::ProvideRuntimeApi;
pub use pallet_quadratic_funding_runtime_api::QuadraticFundingApi as QuadraticFundingRuntimeApi;
pub use self::gen_client::Client as QuadraticFundingClient;


// TODO: There is a bug for serde_json, can not use u128 https://github.com/paritytech/substrate/issues/4641
#[rpc]
pub trait QuadraticFundingApi<AccountId, Hash> {
	#[rpc(name = "qf_querVoteCost")]
	fn vote_cost(
		&self,
        who: AccountId,
        round_id:u32,
        project_hash: Hash, 
        ballot: u32
	) -> Result<u32>;
}

/// A struct that implements the [`QuadraticFundingApi`].
pub struct QuadraticFunding<C, P> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<P>,
}

impl<C, P> QuadraticFunding<C, P> {
	/// Create new `QuadraticFunding` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

/// Error type of this RPC api.
pub enum Error {
	/// The transaction was not decodable.
	DecodeError,
	/// The call to runtime failed.
	RuntimeError,
}

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
			Error::DecodeError => 2,
		}
	}
}

impl<C, Block, AccountId, Hash> QuadraticFundingApi<
AccountId,
Hash,
> for QuadraticFunding<C, Block>
where
    Block: BlockT,
	C: 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: QuadraticFundingRuntimeApi<Block, AccountId, Hash>,
    AccountId: Clone + MaybeDisplay + Codec,
	Hash: Codec + MaybeDisplay + Copy,
{
	fn vote_cost(
		&self,
        who: AccountId,
        round_id:u32,
        project_hash: Hash, 
        ballot: u32
	) -> Result<u32> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);
		api.vote_cost(&at, who, round_id, project_hash, ballot).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to query dispatch info.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}