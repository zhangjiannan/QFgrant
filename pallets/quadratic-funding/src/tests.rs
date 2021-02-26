use crate::{Error, mock::*};
use frame_support::{assert_ok, assert_noop};
use sp_core::H256;
//use frame_system::RawEvent;
use super::RawEvent;

/// generate a Hash for indexing project
fn get_hash(value: u128) -> H256 {
	let slices = value.to_be_bytes();
	H256::from_slice(&slices.repeat(2))
}

fn last_event() -> RawEvent<u64, H256> {
	System::events().into_iter().map(|r| r.event)
		.filter_map(|e| {
			if let Event::quadratic_funding(inner) = e { Some(inner) } else { None }
		})
		.last()
		.unwrap()
}

#[test]
fn register_project_works() {
	new_test_ext().execute_with(|| {
		// IMPORTANT, event won't emit in block 0
		System::set_block_number(1);
		let hash = get_hash(1);
		let project_name = b"name".to_vec();
		// Dispatch a signed extrinsic.
		assert_ok!(QuadraticFunding::register_project(Origin::signed(1), hash, project_name.clone()));
		// Read pallet storage and assert an expected result.
		// positive case
		assert_eq!(QuadraticFunding::projects(hash).name, project_name);
		// negative case
		assert_noop!(
			QuadraticFunding::register_project(Origin::signed(1), hash, project_name),
			Error::<Test>::DuplicateProject
		);

		assert_eq!(Balances::free_balance(0), 1000);
		assert_ok!(QuadraticFunding::vote_cost(Origin::signed(1), hash, 1));
		assert_eq!(last_event(), RawEvent::VoteCost(hash,1));
	});
}

#[test]
fn donate_works() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		assert_ok!(QuadraticFunding::donate(Origin::signed(1), 500));
		// check the support pool
		assert_eq!(QuadraticFunding::pre_tax_support_pool(), 500);
		// fee rate is 5%
		assert_eq!(QuadraticFunding::total_tax(), 25);
		assert_eq!(QuadraticFunding::support_pool(), 475);		
	});
}
