#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame
/// debug guide https://substrate.dev/recipes/runtime-printing.html

use frame_support::{
	decl_module, decl_storage, decl_event, decl_error, dispatch, debug, ensure,
	traits::{Currency, EnsureOrigin, ReservableCurrency, OnUnbalanced, Get, ExistenceRequirement::{KeepAlive}},
};
use sp_runtime::{ModuleId, traits::{StaticLookup, AccountIdConversion}};
use frame_support::codec::{Encode, Decode};
use frame_system::{ensure_signed, ensure_root};
use sp_std::{vec::Vec, convert::{TryInto}};
// use sp_runtime::traits::CheckedMul;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Project<AccountId> {
	pub total_votes: u128,
	pub grants: u128,
	pub support_area: u128,
	pub withdrew: u128,
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

	/// UnitOfVote, 0.001 Unit token
	type UnitOfVote: Get<u128>;

	/// What to do with slashed funds.
	type Slashed: OnUnbalanced<NegativeImbalanceOf<Self>>;

	/// The origin which may forcibly set or remove a name. Root can always do this.
	type ForceOrigin: EnsureOrigin<Self::Origin>;

	/// Number of base unit for each vote
	type NumberOfUnitPerVote: Get<u128>;

	/// The ration of fee based on the number of unit
	type FeeRatioPerVote: Get<u128>;
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
		Round get(fn rounds): u32;
		// supportPool, preTaxSupportPool, _totalSupportArea
		SupportPool get(fn support_pool): u128;
		PreTaxSupportPool get(fn pre_tax_support_pool): u128;
		TotalSupportArea get(fn total_support_area): u128;
		TotalTax get(fn total_tax): u128;
		Projects get(fn projects): map hasher(blake2_128_concat) T::Hash => ProjectOf<T>;
		ProjectVotes: double_map hasher(blake2_128_concat) T::Hash, hasher(blake2_128_concat) T::AccountId => u128;
	}
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId, Hash =  <T as frame_system::Trait>::Hash, {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		ProjectRegistered(Hash, AccountId),
		VoteCost(Hash, u128),
		VoteSucceed(Hash, AccountId, u128),
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
		DonationTooSmall,
		InvalidRound,
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
		const UnitOfVote: u128 = T::UnitOfVote::get();
		const NumberOfUnitPerVote: u128 = T::NumberOfUnitPerVote::get();
		const FeeRatioPerVote: u128 = T::FeeRatioPerVote::get();

		/// get sponsored
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn donate(origin, #[compact] amount: BalanceOf<T>) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			// the minimum unit, make sure the donate is bigger than this
			let min_unit_number = Self::cal_amount(1u128, false);
			let amount_number = Self::balance_to_u128(amount);
			let fee_number = T::FeeRatioPerVote::get().checked_mul(amount_number / T::NumberOfUnitPerVote::get()).unwrap();
			ensure!(amount_number > min_unit_number, Error::<T>::DonationTooSmall);
			PreTaxSupportPool::mutate(|pool| *pool=amount_number.checked_add(*pool).unwrap());
			SupportPool::mutate(|pool| *pool=(amount_number-fee_number).checked_add(*pool).unwrap());

			TotalTax::mutate(|pool| *pool=fee_number.checked_add(*pool).unwrap());
			let _ = T::Currency::transfer(&who, &Self::account_id(), amount, KeepAlive);
			Ok(())
		}

		// #[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		// pub fn withdraw(origin, beneficiary: <T::Lookup as StaticLookup>::Source, #[compact] amount: BalanceOf<T>) -> dispatch::DispatchResult {
		// 	let who = ensure_signed(origin)?;
		// 	let beneficiary =  T::Lookup::lookup(beneficiary)?;
		// 	let _ = T::Currency::transfer(&Self::account_id(), &who, amount, KeepAlive);

		// 	Ok(())
		// }
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn end_round(origin, round: u32) -> dispatch::DispatchResult {
			//ensure_root(origin)?;
			ensure!(round == Round::get(), Error::<T>::InvalidRound);
			let area = TotalSupportArea::get();
			let pool = SupportPool::get();
			for (hash, mut project) in Projects::<T>::iter() {
				if area > 0 {
					let total = project.grants;
					project.grants = total.checked_add(
						project.support_area.checked_mul(pool/area).unwrap()
					).unwrap();
				}
				debug::info!("Hash: {:?}, Total votes: {:?}, Grants: {:?}", hash, project.total_votes, project.grants);
				// reckon the final grants
				let _ = T::Currency::transfer(
					&Self::account_id(),
					&project.owner,
					Self::u128_to_balance(project.grants),
					KeepAlive
				);
			}
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
			Round::put(1);
			Self::deposit_event(RawEvent::ProjectRegistered(hash, who));
			Ok(())
		}

		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn vote(origin, hash: T::Hash, ballot: u128) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Projects::<T>::contains_key(&hash), Error::<T>::ProjectNotExist);
			ensure!(ballot > 0, Error::<T>::InvalidBallot);
			let voted = ProjectVotes::<T>::get(hash, &who);
			let cost = Self::cal_cost(voted, ballot);
			ProjectVotes::<T>::insert(hash, &who, ballot+voted);
			let amount = Self::cal_amount(cost, false);
			let fee = Self::cal_amount(cost, true);
			Projects::<T>::mutate(hash, |poj| {
				let support_area = ballot.checked_mul(poj.total_votes - voted).unwrap();
				poj.support_area = support_area.checked_add(poj.support_area).unwrap();
				poj.total_votes += ballot;
				poj.grants += amount - fee;
				debug::info!("Total votes: {:?}, Current votes: {:?}, Support Area: {:?},Est cost: {:?}",
				poj.total_votes, voted, support_area, cost);
				TotalSupportArea::mutate(|area| *area=support_area.checked_add(*area).unwrap());
				TotalTax::mutate(|tax| *tax=fee.checked_add(*tax).unwrap());
			});
			let _ = T::Currency::transfer(&who, &Self::account_id(), Self::u128_to_balance(amount), KeepAlive);
			Self::deposit_event(RawEvent::VoteSucceed(hash, who, ballot));
			Ok(())
		}

		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn vote_cost(origin, hash: T::Hash, ballot: u128) -> dispatch::DispatchResult {
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
	pub fn cal_cost(voted: u128, ballot: u128) -> u128 {
		let mut points = ballot.checked_mul(ballot.checked_add(1).unwrap()).unwrap() / 2; 
		points = points.checked_add(ballot.checked_mul(voted).unwrap()).unwrap();
		return points;
	}

	pub fn cal_amount(amount: u128, is_fee: bool) -> u128 {
		let uov = T::UnitOfVote::get();
		let nup = T::NumberOfUnitPerVote::get();
		let frpv = T::FeeRatioPerVote::get();
		if is_fee { 
			uov.checked_mul(frpv).unwrap().checked_mul(amount).unwrap() 
		} else {
			uov.checked_mul(nup).unwrap().checked_mul(amount).unwrap()
		}
	}

	pub fn u128_to_balance(cost: u128) -> BalanceOf<T> {
		TryInto::<BalanceOf::<T>>::try_into(cost).ok().unwrap()
	}

	pub fn balance_to_u128(balance: BalanceOf<T>) -> u128 {
		TryInto::<u128>::try_into(balance).ok().unwrap()
	}
}