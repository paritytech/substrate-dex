//! # Substrate DEX
//!
//! ## Overview
//!
//! This pallet is a port of Uniswap V1 functionality to substrate.
//!
//! ## Interface
//!
//! ### Config
//!
//!
//! ### Dispatchable functions
//!
//!

// TODO: Remove when placeholders are filled
#![allow(unused)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use frame_support::traits::{Currency, ReservableCurrency};
use sp_std::prelude::*;

pub use pallet::*;
pub use weights::WeightInfo;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <T as Config>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_support::sp_runtime::traits::StaticLookup;
    use frame_support::traits::tokens::Balance;
    use frame_support::BoundedBTreeMap;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_assets::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait.
        type Currency: ReservableCurrency<Self::AccountId>
            + IsType<<Self as pallet_assets::Config>::Currency>;

        /// Single balance type for base currency and assets.
        type Balance: IsType<<<Self as Config>::Currency as Currency<AccountIdOf<Self>>>::Balance>
            + IsType<<Self as pallet_assets::Config>::Balance>
            + Balance
            + MaxEncodedLen;

        /// Maximum number of liquidity providers per exchange.
        type MaxExchangeProviders: Get<u32> + TypeInfo;

        /// Information on runtime weights.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // TODO
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Asset with the specified ID does not exist
        AssetNotFound,
    }

    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, MaxEncodedLen, TypeInfo,
    )]
    pub struct Exchange<AssetId, AccountId: Ord, Balance, MaxProviders: Get<u32>> {
        pub asset_id: AssetId,
        pub total_supply: Balance,
        pub balances: BoundedBTreeMap<AccountId, Balance, MaxProviders>,
    }

    type ExchangeOf<T> = Exchange<
        <T as pallet_assets::Config>::AssetId,
        AccountIdOf<T>,
        BalanceOf<T>,
        <T as Config>::MaxExchangeProviders,
    >;

    #[pallet::storage]
    #[pallet::getter(fn exchanges)]
    pub(super) type Exchanges<T: Config> =
        StorageMap<_, Twox64Concat, T::AssetId, ExchangeOf<T>, OptionQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new exchange.
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn create_exchange(origin: OriginFor<T>, asset_id: T::AssetId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // TODO: Fee/deposit for exchange creation (?)

            if <pallet_assets::Pallet<T>>::maybe_total_supply(asset_id).is_none() {
                Err(Error::<T>::AssetNotFound)?
            }

            <Exchanges<T>>::insert(
                asset_id,
                Exchange {
                    asset_id,
                    total_supply: <BalanceOf<T>>::default(),
                    balances: BoundedBTreeMap::new(),
                },
            );

            Ok(())
        }

        #[pallet::weight(1000)]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            currency_amount: BalanceOf<T>,
            min_liquidity: BalanceOf<T>,
            max_tokens: BalanceOf<T>,
        ) -> DispatchResult {
            unimplemented!()
        }
    }
}
