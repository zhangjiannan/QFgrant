use crate::{Error, mock::*};
use frame_support::{assert_ok, assert_noop};
use sp_core::H256;

/// generate a Hash for indexing project
fn get_hash(value: u128) -> H256 {
	let slices = value.to_be_bytes();
	H256::from_slice(&slices.repeat(2))
}
type Balances = pallet_balances::Module<Test>;

#[test]
fn qf_register_project() {
	new_test_ext().execute_with(|| {
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

		assert_eq!(Balances::free_balance(0), 100);
	});
}