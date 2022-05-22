#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod types;

use frame_support::ensure;
use sp_std::vec::Vec;
use types::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + scale_info::TypeInfo {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	#[pallet::storage]
	#[pallet::getter(fn asset)]
	/// Details of an asset.
	pub(super) type Asset<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, AssetDetails<T>>;

	#[pallet::storage]
	#[pallet::getter(fn account)]
	/// The holdings of a specific account for a specific asset.
	pub(super) type Account<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetId,
		Blake2_128Concat,
		T::AccountId,
		u128,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn metadata)]
	/// Details of an asset.
	pub(super) type Metadata<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetId, types::AssetMetadata>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	/// Nonce for id of the next created asset
	pub(super) type Nonce<T: Config> = StorageValue<_, AssetId, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New asset created
		Created {
			owner: T::AccountId,
			asset_id: AssetId,
		},
		/// New metadata has been set for an asset
		MetadataSet {
			asset_id: AssetId,
			name: Vec<u8>,
			symbol: Vec<u8>,
		},
		/// Some assets have been minted
		Minted {
			asset_id: AssetId,
			owner: T::AccountId,
			total_supply: u128,
		},
		/// Some assets have been burned
		Burned {
			asset_id: AssetId,
			owner: T::AccountId,
			total_supply: u128,
		},
		/// Some assets have been transferred
		Transferred {
			asset_id: AssetId,
			from: T::AccountId,
			to: T::AccountId,
			amount: u128,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// The asset ID is unknown
		Unknown,
		/// The signing account has no permision to do the operation
		NoPermission,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn create(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			let id = Self::nonce();
			let details = AssetDetails::new(origin.clone());

			Asset::<T>::insert(id, details);
			Nonce::<T>::set(id.saturating_add(1));

			Self::deposit_event(Event::<T>::Created {
				owner: origin,
				asset_id: id,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_metadata(
			origin: OriginFor<T>,
			asset_id: AssetId,
			name: Vec<u8>,
			symbol: Vec<u8>,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			Self::ensure_is_owner(asset_id, origin)?;

			// TODO:
			// - create a new AssetMetadata instance based on the call arguments
			let asset_metadata = AssetMetadata::new(name.clone(), symbol.clone());

			// - insert this metadata in the Metadata storage, under the asset_id key
			Metadata::<T>::insert(asset_id, asset_metadata);

			// - deposit an `Created` event
			Self::deposit_event(Event::<T>::MetadataSet {
				asset_id,
				name,
				symbol,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn mint(
			origin: OriginFor<T>,
			asset_id: AssetId,
			amount: u128,
			to: T::AccountId,
		) -> DispatchResult {
			// TODO:
			// - ensure the extrinsic origin is a signed transaction
			let who = ensure_signed(origin)?;

			// - ensure the caller is the asset owner
			Self::ensure_is_owner(asset_id.clone(), who.clone())?;

			let mut minted_amount = 0;
			let mut total_supply = 0;

			Asset::<T>::try_mutate(asset_id, |maybe_details| -> DispatchResult {
				let details = maybe_details.as_mut().ok_or(Error::<T>::Unknown)?;

				let old_supply = details.supply;
				details.supply = details.supply.saturating_add(amount);
				total_supply = details.supply;
				minted_amount = details.supply - old_supply;

				Ok(())
			})?;

			Account::<T>::mutate(asset_id, to.clone(), |balance| {
				*balance += minted_amount;
			});

			// TODO: Deposit a `Minted` event
			Self::deposit_event(Event::<T>::Minted {
				asset_id,
				owner: to,
				total_supply,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn burn(origin: OriginFor<T>, asset_id: AssetId, amount: u128) -> DispatchResult {
			// TODO:
			// - ensure the extrinsic origin is a signed transaction
			let who = ensure_signed(origin)?;

			let mut new_total_supply = 0;

			// - mutate the total supply
			Asset::<T>::try_mutate(asset_id, |maybe_details| -> DispatchResult {
				// Get access to `AssetDetails`
				let details = maybe_details.as_mut().ok_or(Error::<T>::Unknown)?;

				let mut burned_amount = 0;

				// - mutate the account balance
				Account::<T>::try_mutate(asset_id, who.clone(), |balance| -> DispatchResult {
					let old_balance = *balance;
					*balance = balance.saturating_sub(amount);
					burned_amount = old_balance - *balance;
					Ok(())
				})?;

				details.supply -= burned_amount;
				new_total_supply = details.supply;

				Ok(())
			})?;

			// - emit a `Burned` event
			Self::deposit_event(Event::<T>::Burned {
				asset_id,
				owner: who,
				total_supply: new_total_supply,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			asset_id: AssetId,
			amount: u128,
			to: T::AccountId,
		) -> DispatchResult {
			// TODO:
			// - ensure the extrinsic origin is a signed transaction
			let who = ensure_signed(origin)?;

			// Ensure asset is valid
			ensure!(Self::asset(asset_id).is_some(), Error::<T>::Unknown);

			// - mutate both account balance
			let mut transferred_from_source = 0;
			let mut transferred_to_dest = 0;

			Account::<T>::try_mutate(asset_id, to.clone(), |to_balance| -> DispatchResult {
				// Subtract `amount` from source account. If `amount` > source account balance, subtract entire source account balance
				Account::<T>::try_mutate(
					asset_id,
					who.clone(),
					|from_balance| -> DispatchResult {
						let old_balance = *from_balance;
						*from_balance = old_balance.saturating_sub(amount);
						transferred_from_source = old_balance - *from_balance;

						Ok(())
					},
				)?;

				// Add `transferred_from_source` to destination account balance
				let old_balance = *to_balance;
				*to_balance = to_balance.saturating_add(transferred_from_source);
				transferred_to_dest = *to_balance - old_balance;

				Ok(())
			})?;

			// - emit a `Transfered` event
			Self::deposit_event(Event::<T>::Transferred {
				asset_id,
				from: who,
				to,
				amount: transferred_to_dest,
			});

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	// This is not a call, so it cannot be called directly by real world users
	// Still it have to be generic over the runtime types, that's why we implement it on Pallet rather than just defining a local function
	fn ensure_is_owner(asset_id: AssetId, account: T::AccountId) -> Result<(), Error<T>> {
		let details = Self::asset(asset_id).ok_or(Error::<T>::Unknown)?;
		ensure!(details.owner == account, Error::<T>::NoPermission);

		Ok(())
	}
}
