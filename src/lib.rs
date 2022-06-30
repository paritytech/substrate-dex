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
    use frame_support::sp_runtime::traits::{StaticLookup, Zero};
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

        // FIXME: Remove this and allow different types for currency and assets
        /// Single balance type for base currency and assets.
        type Balance: IsType<<<Self as Config>::Currency as Currency<AccountIdOf<Self>>>::Balance>
            + IsType<<Self as pallet_assets::Config>::Balance>
            + Balance
            + MaxEncodedLen;

        /// Maximum number of liquidity providers per exchange.
        #[pallet::constant]
        type MaxExchangeProviders: Get<u32>;

        /// Information on runtime weights.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new exchange was created [asset_id]
        ExchangeCreated(T::AssetId),
        /// Liquidity was added to an exchange [provider_id, asset_id, currency_amount, token_amount, liquidity_minted]
        LiquidityAdded(
            T::AccountId,
            T::AssetId,
            BalanceOf<T>,
            BalanceOf<T>,
            BalanceOf<T>,
        ),
        /// Liquidity was removed from an exchange [provider_id, asset_id, currency_amount, token_amount, liquidity_amount]
        LiquidityRemoved(
            T::AccountId,
            T::AssetId,
            BalanceOf<T>,
            BalanceOf<T>,
            BalanceOf<T>,
        ),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Asset with the specified ID does not exist
        AssetNotFound,
        /// Exchange for the given asset already exists
        ExchangeAlreadyExists,
        /// Not enough free balance to add liquidity
        BalanceTooLow,
        /// Not enough tokens to add liquidity
        NotEnoughTokens,
        /// Zero value provided for `max_tokens` parameter
        MaxTokensIsZero,
        /// Zero value provided for `currency_amount` parameter
        CurrencyAmountIsZero,
        /// Zero value provided for `min_liquidity` parameter
        MinLiquidityIsZero,
        /// No exchange found for the given `asset_id`
        ExchangeNotFound,
        /// Specified `max_tokens` is too low to match `currency_amount`
        MaxTokensTooLow,
        /// Specified `min_liquidity` is too high to match `currency_amount`
        MinLiquidityTooHigh,
        /// Maximum number of liquidity providers for the exchange reached
        MaxProvidersReached,
        /// Zero value provided for `liquidity_amount` parameter
        LiquidityAmountIsZero,
        /// Zero value provided for `min_currency` parameter
        MinCurrencyIsZero,
        /// Zero value provided for `min_tokens` parameter
        MinTokensIsZero,
        /// There's not enough total liquidity in the exchange
        TotalLiquidityTooLow,
        /// Specified account doesn't own enough liquidity in the exchange
        ProviderLiquidityTooLow,
        /// Specified account doesn't provide any liquidity in the exchange
        NotAProvider,
        /// Withdrawn liquidity is not sufficient for specified `min_currency`
        MinCurrencyTooHigh,
        /// Withdrawn liquidity is not sufficient for specified `min_tokens`
        MinTokensTooHigh,
    }

    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, MaxEncodedLen, TypeInfo,
    )]
    pub struct Exchange<AssetId, Balance, BalanceMap> {
        pub asset_id: AssetId,
        pub total_liquidity: Balance,
        pub currency_reserve: Balance,
        pub token_reserve: Balance,
        pub balances: BalanceMap,
    }

    // Type aliases for convenience
    type BalanceMap<T> =
        BoundedBTreeMap<AccountIdOf<T>, BalanceOf<T>, <T as Config>::MaxExchangeProviders>;
    type ExchangeOf<T> =
        Exchange<<T as pallet_assets::Config>::AssetId, BalanceOf<T>, BalanceMap<T>>;

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
            if <Exchanges<T>>::contains_key(asset_id) {
                Err(Error::<T>::ExchangeAlreadyExists)?
            }

            <Exchanges<T>>::insert(
                asset_id,
                Exchange {
                    asset_id,
                    total_liquidity: <BalanceOf<T>>::default(),
                    currency_reserve: <BalanceOf<T>>::default(),
                    token_reserve: <BalanceOf<T>>::default(),
                    balances: BoundedBTreeMap::new(),
                },
            );

            Self::deposit_event(Event::ExchangeCreated(asset_id));
            Ok(())
        }

        /// Add liquidity to an exchange.
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            currency_amount: BalanceOf<T>,
            min_liquidity: BalanceOf<T>,
            max_tokens: BalanceOf<T>,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            let caller = ensure_signed(origin)?;
            ensure!(
                currency_amount > 0u32.into(),
                Error::<T>::CurrencyAmountIsZero
            );
            ensure!(max_tokens > 0u32.into(), Error::<T>::MaxTokensIsZero);
            ensure!(
                <T as Config>::Currency::free_balance(&caller) >= currency_amount.into(),
                Error::<T>::BalanceTooLow
            );
            match <pallet_assets::Pallet<T>>::maybe_balance(asset_id, &caller) {
                None => Err(Error::<T>::AssetNotFound)?,
                Some(balance) => ensure!(balance >= max_tokens.into(), Error::<T>::NotEnoughTokens),
            }
            let mut exchange = match <Exchanges<T>>::get(asset_id) {
                Some(exchange) => exchange,
                None => Err(Error::<T>::ExchangeNotFound)?,
            };
            let caller_liquidity = match exchange.balances.get_mut(&caller) {
                Some(balance) => balance,
                None => {
                    exchange
                        .balances
                        .try_insert(caller.clone(), 0u32.into())
                        .map_err(|_| Error::<T>::MaxProvidersReached)?;
                    exchange.balances.get_mut(&caller).unwrap()
                }
            };

            // -------------------- Token/liquidity computation --------------------
            let (token_amount, liquidity_minted) = if exchange.total_liquidity > 0u32.into() {
                ensure!(min_liquidity > 0u32.into(), Error::<T>::MinLiquidityIsZero);
                let token_amount = currency_amount * exchange.token_reserve
                    / exchange.currency_reserve
                    + 1u32.into();
                let liquidity_minted =
                    currency_amount * exchange.total_liquidity / exchange.currency_reserve;
                ensure!(token_amount <= max_tokens, Error::<T>::MaxTokensTooLow);
                ensure!(
                    liquidity_minted >= min_liquidity,
                    Error::<T>::MinLiquidityTooHigh
                );
                (token_amount, liquidity_minted)
            } else {
                (max_tokens, currency_amount)
            };

            // --------------------- Currency & token transfer ---------------------
            // TODO: Derive account from pallet and make transfers

            // -------------------------- Balances update --------------------------
            exchange.currency_reserve += currency_amount;
            exchange.token_reserve += token_amount;
            exchange.total_liquidity += liquidity_minted;
            *caller_liquidity += liquidity_minted;
            <Exchanges<T>>::insert(asset_id, exchange);

            // ---------------------------- Emit event -----------------------------
            Self::deposit_event(Event::LiquidityAdded(
                caller,
                asset_id,
                currency_amount,
                token_amount,
                liquidity_minted,
            ));
            Ok(())
        }

        /// Remove liquidity from an exchange.
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn remove_liquidity(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            liquidity_amount: BalanceOf<T>,
            min_currency: BalanceOf<T>,
            min_tokens: BalanceOf<T>,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            let caller = ensure_signed(origin)?;
            ensure!(
                liquidity_amount > 0u32.into(),
                Error::<T>::LiquidityAmountIsZero
            );
            ensure!(min_currency > 0u32.into(), Error::<T>::MinCurrencyIsZero);
            ensure!(min_tokens > 0u32.into(), Error::<T>::MinTokensIsZero);
            let mut exchange = match <Exchanges<T>>::get(asset_id) {
                Some(exchange) => exchange,
                None => Err(Error::<T>::ExchangeNotFound)?,
            };
            ensure!(
                exchange.total_liquidity >= liquidity_amount,
                Error::<T>::TotalLiquidityTooLow
            );
            let caller_liquidity = exchange
                .balances
                .get_mut(&caller)
                .ok_or(Error::<T>::NotAProvider)?;
            ensure!(
                *caller_liquidity >= liquidity_amount,
                Error::<T>::ProviderLiquidityTooLow
            );

            // --------------- Withdrawn currency/tokens computation ---------------
            let currency_amount =
                liquidity_amount * exchange.currency_reserve / exchange.total_liquidity;
            let token_amount = liquidity_amount * exchange.token_reserve / exchange.total_liquidity;
            ensure!(
                currency_amount >= min_currency,
                Error::<T>::MinCurrencyTooHigh
            );
            ensure!(token_amount >= min_tokens, Error::<T>::MinTokensTooHigh);

            // --------------------- Currency & token transfer ---------------------
            // TODO: Derive account from pallet and make transfers

            // -------------------------- Balances update --------------------------
            exchange.currency_reserve -= currency_amount;
            exchange.token_reserve -= token_amount;
            exchange.total_liquidity -= liquidity_amount;
            *caller_liquidity -= liquidity_amount;
            if caller_liquidity.is_zero() {
                exchange.balances.remove(&caller);
            }

            // ---------------------------- Emit event -----------------------------
            Self::deposit_event(Event::LiquidityRemoved(
                caller,
                asset_id,
                currency_amount,
                token_amount,
                liquidity_amount,
            ));
            Ok(())
        }
    }
}
