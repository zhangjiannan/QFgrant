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
		assert_ok!(QuadraticFunding::donate(Origin::signed(0), 500));
		assert_eq!(Balances::free_balance(0), 500);
		// check the support pool
		assert_eq!(QuadraticFunding::pre_tax_support_pool(), 500);
		// fee rate is 5%
		assert_eq!(QuadraticFunding::total_tax(), 25);
		assert_eq!(QuadraticFunding::support_pool(), 475);		
	});
}

#[test]
fn vote_without_fund_works() {
	new_test_ext().execute_with(|| {
		// initalize 3 projects
		for i in 1..4 {
			System::set_block_number(i);
			let hash = get_hash(i.into());
			let project_name = b"name".to_vec();
			assert_ok!(QuadraticFunding::register_project(Origin::signed(i), hash, project_name.clone()));
			assert_eq!(QuadraticFunding::projects(hash).name, project_name);

			// vote for each own's project only once, in this case there will be no fund
			let vote = 3;
			let expected_cost:u64 = vote * (vote + 1) / 2 * 100;
			assert_ok!(QuadraticFunding::vote(Origin::signed(i), hash, vote.into()));
			// We initialize the balance sequentially, each one got 1000*(i+1) pico
			assert_eq!(Balances::free_balance(i), 1000*(i+1) - expected_cost);
		}
		assert_ok!(QuadraticFunding::end_round(Origin::signed(0), 1));
		// no support area means no fund expense
		assert_eq!(QuadraticFunding::total_support_area(), 0);
	});
}


#[test]
fn vote_with_fund_works() {
	new_test_ext().execute_with(|| {
		// sponsor default round
		assert_ok!(QuadraticFunding::donate(Origin::signed(0), 500));
		// initalize 3 projects
		for i in 1..4 {
			System::set_block_number(i);
			let hash = get_hash(i.into());
			let project_name = b"name".to_vec();
			assert_ok!(QuadraticFunding::register_project(Origin::signed(i), hash, project_name.clone()));
			assert_eq!(QuadraticFunding::projects(hash).name, project_name);

			// vote to each other, the area should be 3,3,12
			for j in 1..4 {
				let vote = if i > 2 {2} else {1};
				assert_ok!(QuadraticFunding::vote(Origin::signed(j), hash, vote));
			}
		}
		assert_eq!(QuadraticFunding::projects(get_hash(1)).support_area, 3);
		assert_eq!(QuadraticFunding::projects(get_hash(3)).support_area, 12);
		// total area is 18
		assert_eq!(QuadraticFunding::total_support_area(), 18);
		
	});
}