#![cfg_attr(not(feature = "std"), no_std)]

// Here we declare the runtime API. It is implemented it the `impl` block in
// runtime amalgamator file (the `runtime/src/lib.rs`)
use codec::{self, Codec, Encode};
use sp_runtime::traits::MaybeDisplay;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
	pub trait QuadraticFundingApi<AccountId, Hash> where
	AccountId: Clone + MaybeDisplay + Encode,
	Hash: Codec + MaybeDisplay
	{
		fn vote_cost(who: AccountId, round_id:u32, hash: Hash, ballot: u32) -> u32;
		fn projects_per_round(round_id:u32) -> Vec<(Hash, u32, u32, u32)>;
	}
}