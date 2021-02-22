#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame
/// debug guide https://substrate.dev/recipes/runtime-printing.html

use frame_support::{
	decl_module, decl_storage, decl_event, decl_error, dispatch, debug, ensure,
	traits::{Currency, EnsureOrigin, ReservableCurrency, OnUnbalanced, Get, ExistenceRequirement::{KeepAlive, AllowDeath}},
};
use sp_runtime::{Permill, ModuleId, Percent, RuntimeDebug, DispatchResult, traits::{
	Zero, StaticLookup, AccountIdConversion, Saturating, Hash, BadOrigin
}};
use frame_support::codec::{Encode, Decode};
use frame_system::ensure_signed;
use sp_std::vec::Vec;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Project<AccountId> {
	pub total_votes: u32,
	pub grants: u32,
	pub support_area: u32,
	pub withdrew: u32,
	pub name: Vec<u8>,
	pub round: u32,
	pub owner: AccountId,
}

type ProjectOf<T> = Project<<T as frame_system::Trait>::AccountId>;
type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
type NegativeImbalanceOf<T> = <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::NegativeImbalance;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Trait: frame_system::Trait {
	// used to generate sovereign account
	// refer: https://github.com/paritytech/substrate/blob/743accbe3256de2fc615adcaa3ab03ebdbbb4dbd/frame/treasury/src/lib.rs#L92
	type ModuleId: Get<ModuleId>;

    // The runtime must supply this pallet with an Event type that satisfies the pallet's requirements.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// The currency trait.
	type Currency: ReservableCurrency<Self::AccountId>;

	/// Reservation fee.
	type ReservationFee: Get<BalanceOf<Self>>;

	/// What to do with slashed funds.
	type Slashed: OnUnbalanced<NegativeImbalanceOf<Self>>;

	/// The origin which may forcibly set or remove a name. Root can always do this.
	type ForceOrigin: EnsureOrigin<Self::Origin>;

	/// The minimum length a name may be.
	type MinLength: Get<usize>;

	/// The maximum length a name may be.
	type MaxLength: Get<usize>;
}

// The pallet's runtime storage items.
// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
	// A unique name is used to ensure that the pallet's storage items are isolated.
	// This name may be updated, but each pallet in the runtime must use a unique name.
	// ---------------------------------vvvvvvvvvvvvvv
	trait Store for Module<T: Trait> as QuadraticFundingModule {
		// Learn more about declaring storage items:
		// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
		Something get(fn something): Option<u32>;
		Round get(fn rounds): u64;
		Projects get(fn projects): map hasher(blake2_128_concat) T::Hash => ProjectOf<T>;
		ProjectVotes: double_map hasher(blake2_128_concat) T::Hash, hasher(blake2_128_concat) T::AccountId => u32;
	}
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId, Hash =  <T as frame_system::Trait>::Hash, {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		ProjectRegistered(Hash, AccountId),
		VoteCost(Hash, u32),
		VoteSucceed(Hash, AccountId, u32),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		DuplicateProject,
		ProjectNotExist,
		InvalidBallot,
	}
}

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;
		const ReservationFee: BalanceOf<T> = T::ReservationFee::get();

		/// An example of transfer
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn trans(origin, beneficiary: <T::Lookup as StaticLookup>::Source, #[compact] amount: BalanceOf<T>) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			let beneficiary =  T::Lookup::lookup(beneficiary)?;
			let _ = T::Currency::transfer(&who, &Self::account_id(), amount, KeepAlive);

			Ok(())
		}

		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn withdraw(origin, beneficiary: <T::Lookup as StaticLookup>::Source, #[compact] amount: BalanceOf<T>) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			let beneficiary =  T::Lookup::lookup(beneficiary)?;
			let _ = T::Currency::transfer(&Self::account_id(), &who, amount, KeepAlive);

			Ok(())
		}

		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn register_project(origin, hash: T::Hash, name: Vec<u8>) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			let project = Project {
				total_votes: 0,
				grants: 0,
				support_area: 0,
				withdrew: 0,
				name: name,
				round: 1,
				owner: who.clone(),
			};
			ensure!(!Projects::<T>::contains_key(&hash), Error::<T>::DuplicateProject);
			Projects::<T>::insert(hash, project);
			Self::deposit_event(RawEvent::ProjectRegistered(hash, who));
			Ok(())
		}

		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn vote(origin, hash: T::Hash, ballot: u32) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Projects::<T>::contains_key(&hash), Error::<T>::ProjectNotExist);
			ensure!(ballot > 0, Error::<T>::InvalidBallot);
			//ProjectVotes::<T>::insert(hash, &who, ballot);
			let voted = ProjectVotes::<T>::get(hash, &who);
			let cost = Self::cal_cost(voted, ballot);
			debug::info!("Current votes: {:?}", voted);
			debug::info!("Est cost: {:?}", cost);
			ProjectVotes::<T>::insert(hash, &who, ballot+voted);
			// Projects::<T>::mutate(hash, |poj| -> DispatchResult {
			// 	let mut project = poj;
			// 	project.total_votes += ballot;
			// 	Ok(())
			// })?;
			Projects::<T>::mutate(hash, |poj| {
				poj.total_votes += ballot;	
			});
			let amount = cost.checked_mul(100).unwrap().into();
			T::Currency::transfer(&who, &Self::account_id(), amount, KeepAlive);
			Self::deposit_event(RawEvent::VoteSucceed(hash, who, ballot));
			Ok(())
		}

		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn vote_cost(origin, hash: T::Hash, ballot: u32) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Projects::<T>::contains_key(&hash), Error::<T>::ProjectNotExist);
			ensure!(ballot > 0, Error::<T>::InvalidBallot);
			let voted = ProjectVotes::<T>::get(hash, &who);
			let cost = Self::cal_cost(voted, ballot);
			Self::deposit_event(RawEvent::VoteCost(hash, cost));
			Ok(())
		}
	}
}

impl<T: Trait> Module<T> {
	// Add public immutables and private mutables.

	/// refer https://github.com/paritytech/substrate/blob/743accbe3256de2fc615adcaa3ab03ebdbbb4dbd/frame/treasury/src/lib.rs#L351
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}
	 pub fn cal_cost(voted: u32, ballot: u32) -> u32 {
		let mut points = ballot.checked_mul(ballot.checked_add(1).unwrap()).unwrap() / 2; 
		points = points.checked_add(ballot.checked_mul(voted).unwrap()).unwrap();
		return points;
	 }
}