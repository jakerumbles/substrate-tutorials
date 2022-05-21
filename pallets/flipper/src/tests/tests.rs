use super::mock::*;
use crate::Error;
use frame_support::{
	assert_noop, assert_ok, error::BadOrigin, serde::__private::ser::FlatMapSerializeMap,
};
use sp_runtime::DispatchError;

#[test]
fn set_value_ok() {
	new_test_ext().execute_with(|| {
		// TODO: ensure the good behaviour of set_value() function.
		assert_ok!(FlipperModule::set_value(Origin::signed(1), true));
		assert_eq!(FlipperModule::value(), Some(true));
	});
}

#[test]
fn set_value_err_already_set() {
	new_test_ext().execute_with(|| {
		// TODO: verify if the function returns the expected error.
		assert_ok!(FlipperModule::set_value(Origin::signed(1), true));

		assert_noop!(
			FlipperModule::set_value(Origin::signed(1), true),
			Error::<Test>::AlreadySet
		);
	});
}

#[test]
fn flip_value_ok() {
	new_test_ext().execute_with(|| {
		// TODO: ensure the good behaviour of flip_value() function.
		assert_ok!(FlipperModule::set_value(Origin::signed(1), true));
		assert_eq!(FlipperModule::value(), Some(true));
		assert_ok!(FlipperModule::flip_value(Origin::signed(1)));
		assert_eq!(FlipperModule::value(), Some(false));
	})
}

// TODO: Make another test to check the behaviour in the case where an error occured in the
// flip_function().

/// Test that `flip_value()` fails if value has not already been set with `set_value()`
#[test]
fn flip_value_err() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			FlipperModule::flip_value(Origin::signed(1)),
			Error::<Test>::NoneValue
		);
	})
}

/// Test that `set_value()` fails if extrinsic is not signed
#[test]
fn set_value_not_signed() {
	new_test_ext().execute_with(|| {
		assert_noop!(FlipperModule::set_value(Origin::none(), true), BadOrigin);
	})
}
