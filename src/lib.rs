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

use frame_support::traits::Currency;
use sp_std::prelude::*;

pub use pallet::*;
pub use weights::WeightInfo;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
type AssetIdOf<T> = <T as Config>::AssetId;
type AssetBalanceOf<T> = <T as Config>::AssetBalance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use codec::EncodeLike;
    use frame_support::pallet_prelude::*;
    use frame_support::sp_runtime::traits::{
        AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Convert, StaticLookup,
        Zero,
    };
    use frame_support::traits::fungibles::{Inspect, Mutate, Transfer};
    use frame_support::traits::tokens::{Balance, WithdrawConsequence};
    use frame_support::traits::{ExistenceRequirement, OriginTrait, WithdrawReasons};
    use frame_support::{BoundedBTreeMap, PalletId};
    use frame_system::pallet_prelude::*;
    use std::fmt::Debug;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Pallet ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait.
        type Currency: Currency<Self::AccountId>;

        /// The balance type for assets (i.e. tokens).
        type AssetBalance: Balance + MaxEncodedLen;

        // Two-way conversion between asset and currency balances
        type AssetToCurrencyBalance: Convert<Self::AssetBalance, BalanceOf<Self>>;
        type CurrencyToAssetBalance: Convert<BalanceOf<Self>, Self::AssetBalance>;

        /// The asset ID type.
        type AssetId: MaybeSerializeDeserialize
            + MaxEncodedLen
            + TypeInfo
            + Clone
            + Debug
            + PartialEq
            + EncodeLike
            + Decode;

        /// The fungible assets trait.
        type Assets: Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::AssetBalance>
            + Transfer<Self::AccountId>
            + Mutate<Self::AccountId>;

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
        ExchangeCreated(AssetIdOf<T>),
        /// Liquidity was added to an exchange [provider_id, asset_id, currency_amount, token_amount, liquidity_minted]
        LiquidityAdded(
            T::AccountId,
            AssetIdOf<T>,
            BalanceOf<T>,
            AssetBalanceOf<T>,
            AssetBalanceOf<T>,
        ),
        /// Liquidity was removed from an exchange [provider_id, asset_id, currency_amount, token_amount, liquidity_amount]
        LiquidityRemoved(
            T::AccountId,
            AssetIdOf<T>,
            BalanceOf<T>,
            AssetBalanceOf<T>,
            AssetBalanceOf<T>,
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
        /// Overflow occurred
        Overflow,
        /// Underflow occurred
        Underflow,
    }

    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, MaxEncodedLen, TypeInfo,
    )]
    pub struct Exchange<AssetId, Balance, AssetBalance, BalanceMap> {
        pub asset_id: AssetId,
        pub total_liquidity: AssetBalance,
        pub currency_reserve: Balance,
        pub token_reserve: AssetBalance,
        pub balances: BalanceMap,
    }

    // Type aliases for convenience
    type BalanceMap<T> =
        BoundedBTreeMap<AccountIdOf<T>, AssetBalanceOf<T>, <T as Config>::MaxExchangeProviders>;
    type ExchangeOf<T> = Exchange<AssetIdOf<T>, BalanceOf<T>, AssetBalanceOf<T>, BalanceMap<T>>;

    #[pallet::storage]
    #[pallet::getter(fn exchanges)]
    pub(super) type Exchanges<T: Config> =
        StorageMap<_, Twox64Concat, AssetIdOf<T>, ExchangeOf<T>, OptionQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new exchange.
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn create_exchange(origin: OriginFor<T>, asset_id: AssetIdOf<T>) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // TODO: Fee/deposit for exchange creation (?)

            if T::Assets::total_issuance(asset_id.clone()).is_zero() {
                Err(Error::<T>::AssetNotFound)?
            }
            if <Exchanges<T>>::contains_key(asset_id.clone()) {
                Err(Error::<T>::ExchangeAlreadyExists)?
            }

            <Exchanges<T>>::insert(
                asset_id.clone(),
                Exchange {
                    asset_id: asset_id.clone(),
                    total_liquidity: <AssetBalanceOf<T>>::default(),
                    currency_reserve: <BalanceOf<T>>::default(),
                    token_reserve: <AssetBalanceOf<T>>::default(),
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
            asset_id: AssetIdOf<T>,
            currency_amount: BalanceOf<T>,
            min_liquidity: AssetBalanceOf<T>,
            max_tokens: AssetBalanceOf<T>,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            let caller = ensure_signed(origin)?;
            ensure!(
                currency_amount > Zero::zero(),
                Error::<T>::CurrencyAmountIsZero
            );
            ensure!(max_tokens > Zero::zero(), Error::<T>::MaxTokensIsZero);
            ensure!(
                <T as Config>::Currency::free_balance(&caller) >= currency_amount,
                Error::<T>::BalanceTooLow
            );
            match T::Assets::can_withdraw(asset_id.clone(), &caller, max_tokens) {
                WithdrawConsequence::NoFunds => Err(Error::<T>::NotEnoughTokens)?,
                WithdrawConsequence::WouldDie => Err(Error::<T>::NotEnoughTokens)?,
                WithdrawConsequence::UnknownAsset => Err(Error::<T>::AssetNotFound)?,
                WithdrawConsequence::Underflow => Err(Error::<T>::Underflow)?,
                WithdrawConsequence::Overflow => Err(Error::<T>::Overflow)?,
                WithdrawConsequence::Frozen => Err(Error::<T>::NotEnoughTokens)?,
                WithdrawConsequence::ReducedToZero(_) => Err(Error::<T>::NotEnoughTokens)?,
                WithdrawConsequence::Success => (),
            };
            let mut exchange = match <Exchanges<T>>::get(asset_id.clone()) {
                Some(exchange) => exchange,
                None => Err(Error::<T>::ExchangeNotFound)?,
            };
            let caller_liquidity = match exchange.balances.get_mut(&caller) {
                Some(balance) => balance,
                None => {
                    exchange
                        .balances
                        .try_insert(caller.clone(), Zero::zero())
                        .map_err(|_| Error::<T>::MaxProvidersReached)?;
                    exchange.balances.get_mut(&caller).unwrap()
                }
            };

            // -------------------- Token/liquidity computation --------------------
            let (token_amount, liquidity_minted) = if exchange.total_liquidity > Zero::zero() {
                ensure!(min_liquidity > Zero::zero(), Error::<T>::MinLiquidityIsZero);
                let currency_amount = T::CurrencyToAssetBalance::convert(currency_amount);
                let currency_reserve =
                    T::CurrencyToAssetBalance::convert(exchange.currency_reserve);
                let token_amount = currency_amount
                    .checked_mul(&exchange.token_reserve)
                    .ok_or(Error::<T>::Overflow)?
                    .checked_div(&currency_reserve)
                    .expect("currency_reserve should never be 0 if total_liquidity > 0")
                    .checked_add(&1u32.into())
                    .ok_or(Error::<T>::Overflow)?;
                let liquidity_minted = currency_amount
                    .checked_mul(&exchange.total_liquidity)
                    .ok_or(Error::<T>::Overflow)?
                    .checked_div(&currency_reserve)
                    .expect("currency_reserve should never be 0 if total_liquidity > 0");
                ensure!(token_amount <= max_tokens, Error::<T>::MaxTokensTooLow);
                ensure!(
                    liquidity_minted >= min_liquidity,
                    Error::<T>::MinLiquidityTooHigh
                );
                (token_amount, liquidity_minted)
            } else {
                (
                    max_tokens,
                    T::CurrencyToAssetBalance::convert(currency_amount),
                )
            };

            // --------------------- Currency & token transfer ---------------------
            let pallet_account = T::PalletId::get().into_account_truncating();
            <T as pallet::Config>::Currency::transfer(
                &caller,
                &pallet_account,
                currency_amount,
                ExistenceRequirement::KeepAlive,
            )?;
            T::Assets::transfer(
                asset_id.clone(),
                &caller,
                &pallet_account,
                token_amount,
                true,
            )?;

            // -------------------------- Balances update --------------------------
            exchange.currency_reserve = exchange
                .currency_reserve
                .checked_add(&currency_amount)
                .ok_or(Error::<T>::Overflow)?;
            exchange.token_reserve = exchange
                .token_reserve
                .checked_add(&token_amount)
                .ok_or(Error::<T>::Overflow)?;
            exchange.total_liquidity = exchange
                .total_liquidity
                .checked_add(&liquidity_minted)
                .ok_or(Error::<T>::Overflow)?;
            *caller_liquidity = caller_liquidity
                .checked_add(&liquidity_minted)
                .ok_or(Error::<T>::Overflow)?;
            <Exchanges<T>>::insert(asset_id.clone(), exchange);

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
            asset_id: AssetIdOf<T>,
            liquidity_amount: AssetBalanceOf<T>,
            min_currency: BalanceOf<T>,
            min_tokens: AssetBalanceOf<T>,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            let caller = ensure_signed(origin)?;
            ensure!(
                liquidity_amount > Zero::zero(),
                Error::<T>::LiquidityAmountIsZero
            );
            ensure!(min_currency > Zero::zero(), Error::<T>::MinCurrencyIsZero);
            ensure!(min_tokens > Zero::zero(), Error::<T>::MinTokensIsZero);
            let mut exchange = match <Exchanges<T>>::get(asset_id.clone()) {
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
            let currency_reserve = T::CurrencyToAssetBalance::convert(exchange.currency_reserve);
            let currency_amount = liquidity_amount
                .checked_mul(&currency_reserve)
                .ok_or(Error::<T>::Overflow)?
                .checked_div(&exchange.total_liquidity)
                .expect("total_liquidity > 0 is checked earlier");
            let currency_amount = T::AssetToCurrencyBalance::convert(currency_amount);
            let token_amount = liquidity_amount
                .checked_mul(&exchange.token_reserve)
                .ok_or(Error::<T>::Overflow)?
                .checked_div(&exchange.total_liquidity)
                .expect("total_liquidity > 0 is checked earlier");
            ensure!(
                currency_amount >= min_currency,
                Error::<T>::MinCurrencyTooHigh
            );
            ensure!(token_amount >= min_tokens, Error::<T>::MinTokensTooHigh);

            // --------------------- Currency & token transfer ---------------------
            let pallet_account = T::PalletId::get().into_account_truncating();
            <T as pallet::Config>::Currency::transfer(
                &pallet_account,
                &caller,
                currency_amount,
                ExistenceRequirement::AllowDeath,
            )?;
            T::Assets::transfer(
                asset_id.clone(),
                &pallet_account,
                &caller,
                token_amount,
                false,
            )?;

            // -------------------------- Balances update --------------------------
            exchange.currency_reserve = exchange
                .currency_reserve
                .checked_sub(&currency_amount)
                .ok_or(Error::<T>::Overflow)?;
            exchange.token_reserve = exchange
                .token_reserve
                .checked_sub(&token_amount)
                .ok_or(Error::<T>::Overflow)?;
            exchange.total_liquidity = exchange
                .total_liquidity
                .checked_sub(&liquidity_amount)
                .ok_or(Error::<T>::Overflow)?;
            *caller_liquidity = caller_liquidity
                .checked_sub(&liquidity_amount)
                .ok_or(Error::<T>::Overflow)?;

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
