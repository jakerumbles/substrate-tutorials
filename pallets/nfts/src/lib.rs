#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod tests;
pub mod types;

use frame_support::ensure;
use sp_std::vec::Vec;
use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + scale_info::TypeInfo {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn unique_asset)]
	/// A mapping of UniqueAssetId's to UniqueAssetDetails
	pub(super) type UniqueAsset<T: Config> =
		StorageMap<_, Blake2_128Concat, UniqueAssetId, UniqueAssetDetails<T>>;

	#[pallet::storage]
	#[pallet::getter(fn account)]
	/// The holdings of a specific account for a specific asset.
	pub(super) type Account<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		UniqueAssetId,
		Blake2_128Concat,
		T::AccountId,
		u128,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	/// Nonce for id of the next created asset
	pub(super) type Nonce<T: Config> = StorageValue<_, UniqueAssetId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New unique asset created
		Created {
			creator: T::AccountId,
			asset_id: UniqueAssetId,
		},
		/// Some assets have been burned
		Burned {
			asset_id: UniqueAssetId,
			owner: T::AccountId,
			total_supply: u128,
		},
		/// Some assets have been transferred
		Transferred {
			asset_id: UniqueAssetId,
			from: T::AccountId,
			to: T::AccountId,
			amount: u128,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The asset ID is unknown
		Unknown,
		/// The signing account does not own any amount of this asset
		NotOwned,
		/// Supply must be positive
		NoSupply,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn mint(origin: OriginFor<T>, metadata: Vec<u8>, supply: u128) -> DispatchResult {
			// Ensure signed transaction
			let who = ensure_signed(origin)?;

			// `supply` must be > 0
			ensure!(supply > 0, Error::<T>::NoSupply);

			// Create `UniqueAssetDetails
			let u_asset_details = UniqueAssetDetails::new(who.clone(), metadata, supply);

			// Get current nonce
			let nonce = Self::nonce();

			// Update storage
			UniqueAsset::<T>::insert(nonce, u_asset_details);

			// Update `who`' balance
			Account::<T>::mutate(nonce, who.clone(), |balance| -> DispatchResult {
				*balance = (*balance).saturating_add(supply);

				Ok(())
			})?;

			// Increment nonce
			Nonce::<T>::mutate(|num| {
				*num = *num + 1;
			});

			// Deposit event
			Self::deposit_event(Event::<T>::Created {
				creator: who,
				asset_id: nonce,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn burn(origin: OriginFor<T>, asset_id: UniqueAssetId, amount: u128) -> DispatchResult {
			// Ensure signed transaction
			let who = ensure_signed(origin)?;

			// Must be valid `asset_id`
			ensure!(Self::unique_asset(asset_id).is_some(), Error::<T>::Unknown);
			// Must have non-zero balance
			ensure!(
				Self::account(asset_id, who.clone()) > 0,
				Error::<T>::NotOwned
			);

			let mut user_burned = 0;

			// Burn: Subtract `amount` from `who`'s account
			Account::<T>::mutate(asset_id, who.clone(), |balance| -> DispatchResult {
				let old_balance = *balance;
				*balance = old_balance.saturating_sub(amount);
				user_burned = old_balance - *balance;

				Ok(())
			})?;

			let mut total_supply = 0;

			// Burn: Subtract `user_burned` from total asset supply
			UniqueAsset::<T>::mutate(asset_id, |asset_details| -> DispatchResult {
				let details = asset_details.as_mut().unwrap();
				let old_supply = details.supply;
				details.supply = old_supply.saturating_sub(user_burned);
				total_supply = details.supply;

				Ok(())
			})?;

			// Deposit event
			Self::deposit_event(Event::<T>::Burned {
				asset_id,
				owner: who,
				total_supply,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			asset_id: UniqueAssetId,
			amount: u128,
			to: T::AccountId,
		) -> DispatchResult {
			// Ensure signed transaction
			let who = ensure_signed(origin)?;

			// Must be valid `asset_id`
			ensure!(Self::unique_asset(asset_id).is_some(), Error::<T>::Unknown);
			// Must have non-zero balance
			ensure!(
				Self::account(asset_id, who.clone()) > 0,
				Error::<T>::NotOwned
			);

			let mut transferred_from_source = 0;

			// Subtract `amount` from `who`'s balance
			Account::<T>::mutate(asset_id, who.clone(), |balance| -> DispatchResult {
				let old_balance = *balance;
				*balance = (*balance).saturating_sub(amount);
				transferred_from_source = old_balance - *balance;

				Ok(())
			})?;

			// Add `transfered_from_source` to `to` account balance
			Account::<T>::mutate(asset_id, to.clone(), |balance| -> DispatchResult {
				*balance = (*balance) + transferred_from_source;

				Ok(())
			})?;

			// Deposit event
			Self::deposit_event(Event::<T>::Transferred {
				asset_id,
				from: who,
				to,
				amount,
			});

			Ok(())
		}
	}
}
