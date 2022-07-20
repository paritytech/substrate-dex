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
        AccountIdConversion, CheckedAdd, CheckedMul, CheckedSub, Convert, One, Saturating, Zero,
    };
    use frame_support::sp_runtime::{FixedPointNumber, FixedPointOperand, FixedU128};
    use frame_support::traits::fungibles::{Create, Destroy, Inspect, Mutate, Transfer};
    use frame_support::traits::tokens::{Balance, WithdrawConsequence};
    use frame_support::traits::{ExistenceRequirement, OriginTrait, Randomness, WithdrawReasons};
    use frame_support::{transactional, BoundedBTreeMap, PalletId};
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
        type AssetBalance: Balance + MaxEncodedLen + FixedPointOperand;

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

        /// The type for tradable assets.
        type Assets: Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::AssetBalance>
            + Transfer<Self::AccountId>;

        /// The type for liquidity tokens.
        type AssetRegistry: Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::AssetBalance>
            + Mutate<Self::AccountId>
            + Create<Self::AccountId>
            + Destroy<Self::AccountId>;

        /// Randomness for liquidity token ID generation.
        type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

        /// Information on runtime weights.
        type WeightInfo: WeightInfo;

        /// Provider fee numerator.
        #[pallet::constant]
        type ProviderFeeNumerator: Get<BalanceOf<Self>>;

        /// Provider fee denominator.
        #[pallet::constant]
        type ProviderFeeDenominator: Get<BalanceOf<Self>>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new exchange was created [asset_id, liquidity_token_id]
        ExchangeCreated(AssetIdOf<T>, AssetIdOf<T>),
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
        /// Asset was bought (for currency) [asset_id, buyer_id, recipient_id, currency_amount, token_amount]
        CurrencyTradedForAsset(
            AssetIdOf<T>,
            T::AccountId,
            T::AccountId,
            BalanceOf<T>,
            AssetBalanceOf<T>,
        ),
        /// Asset was sold (for currency) [asset_id, buyer_id, recipient_id, currency_amount, token_amount]
        AssetTradedForCurrency(
            AssetIdOf<T>,
            T::AccountId,
            T::AccountId,
            BalanceOf<T>,
            AssetBalanceOf<T>,
        ),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Asset with the specified ID does not exist
        AssetNotFound,
        /// Exchange for the given asset already exists
        ExchangeAlreadyExists,
        /// Not enough free balance to add liquidity or perform trade
        BalanceTooLow,
        /// Not enough tokens to add liquidity or perform trade
        NotEnoughTokens,
        /// Zero value provided for `max_tokens` parameter
        MaxTokensIsZero,
        /// Zero value provided for `currency_amount` parameter
        CurrencyAmountIsZero,
        /// Value provided for `currency_amount` parameter is too high
        CurrencyAmountTooHigh,
        /// Zero value provided for `min_liquidity` parameter
        MinLiquidityIsZero,
        /// No exchange found for the given `asset_id`
        ExchangeNotFound,
        /// Value provided for `max_tokens` parameter is too low
        MaxTokensTooLow,
        /// Value provided for `min_liquidity` parameter is too high
        MinLiquidityTooHigh,
        /// Zero value provided for `liquidity_amount` parameter
        LiquidityAmountIsZero,
        /// Zero value provided for `min_currency` parameter
        MinCurrencyIsZero,
        /// Zero value provided for `min_tokens` parameter
        MinTokensIsZero,
        /// Specified account doesn't own enough liquidity in the exchange
        ProviderLiquidityTooLow,
        /// Value provided for `min_currency` parameter is too high
        MinCurrencyTooHigh,
        /// Value provided for `min_tokens` parameter is too high
        MinTokensTooHigh,
        /// Zero value provided for `max_currency` parameter
        MaxCurrencyIsZero,
        /// Value provided for `max_currency` parameter is too low
        MaxCurrencyTooLow,
        /// Zero value provided for `token_amount` parameter
        TokenAmountIsZero,
        /// Value provided for `token_amount` parameter is too high
        TokenAmountTooHigh,
        /// There is not enough liquidity in the exchange to perform trade
        NotEnoughLiquidity,
        /// Overflow occurred
        Overflow,
        /// Underflow occurred
        Underflow,
        /// Deadline specified for the operation has passed
        DeadlinePassed,
    }

    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, MaxEncodedLen, TypeInfo,
    )]
    pub struct Exchange<AssetId, Balance, AssetBalance> {
        pub asset_id: AssetId,
        pub currency_reserve: Balance,
        pub token_reserve: AssetBalance,
        pub liquidity_token_id: AssetId,
    }

    // Type alias for convenience
    type ExchangeOf<T> = Exchange<AssetIdOf<T>, BalanceOf<T>, AssetBalanceOf<T>>;

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
            // -------------------------- Validation part --------------------------
            let caller = ensure_signed(origin)?;
            // TODO: Fee/deposit for exchange creation (?)

            if T::Assets::total_issuance(asset_id.clone()).is_zero() {
                Err(Error::<T>::AssetNotFound)?
            }
            if <Exchanges<T>>::contains_key(asset_id.clone()) {
                Err(Error::<T>::ExchangeAlreadyExists)?
            }

            // ----------------------- Create liquidity token ----------------------
            let random_hash = T::Randomness::random("liquidity_token_id".as_bytes()).0;
            let liquidity_token_id = <AssetIdOf<T>>::decode(&mut random_hash.as_ref())
                .expect("asset ID shouldn't have more bytes than hash");
            let pallet_account = T::PalletId::get().into_account_truncating();
            T::AssetRegistry::create(
                liquidity_token_id.clone(),
                pallet_account,
                false,
                <AssetBalanceOf<T>>::one(),
            )?;

            // -------------------------- Update storage ---------------------------
            <Exchanges<T>>::insert(
                asset_id.clone(),
                Exchange {
                    asset_id: asset_id.clone(),
                    currency_reserve: <BalanceOf<T>>::zero(),
                    token_reserve: <AssetBalanceOf<T>>::zero(),
                    liquidity_token_id: liquidity_token_id.clone(),
                },
            );

            // ---------------------------- Emit event -----------------------------
            Self::deposit_event(Event::ExchangeCreated(asset_id, liquidity_token_id));
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
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            let caller = ensure_signed(origin)?;
            Self::check_deadline(&deadline)?;
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
                WithdrawConsequence::Success => (),
                WithdrawConsequence::UnknownAsset => Err(Error::<T>::AssetNotFound)?,
                _ => Err(Error::<T>::NotEnoughTokens)?,
            };
            let mut exchange = Self::get_exchange(&asset_id)?;

            // -------------------- Token/liquidity computation --------------------
            let total_liquidity = T::Assets::total_issuance(exchange.liquidity_token_id.clone());
            let (token_amount, liquidity_minted) = if total_liquidity > Zero::zero() {
                ensure!(min_liquidity > Zero::zero(), Error::<T>::MinLiquidityIsZero);
                let currency_amount = T::CurrencyToAssetBalance::convert(currency_amount);
                let currency_reserve =
                    T::CurrencyToAssetBalance::convert(exchange.currency_reserve);
                let token_amount =
                    FixedU128::saturating_from_rational(currency_amount, currency_reserve)
                        .saturating_mul_int(exchange.token_reserve)
                        .saturating_add(One::one());
                let liquidity_minted =
                    FixedU128::saturating_from_rational(currency_amount, currency_reserve)
                        .saturating_mul_int(total_liquidity);
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
            T::AssetRegistry::mint_into(
                exchange.liquidity_token_id.clone(),
                &caller,
                liquidity_minted,
            )?;

            // -------------------------- Balances update --------------------------
            exchange.currency_reserve.saturating_accrue(currency_amount);
            exchange.token_reserve.saturating_accrue(token_amount);
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
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            let caller = ensure_signed(origin)?;
            Self::check_deadline(&deadline)?;
            ensure!(
                liquidity_amount > Zero::zero(),
                Error::<T>::LiquidityAmountIsZero
            );
            ensure!(min_currency > Zero::zero(), Error::<T>::MinCurrencyIsZero);
            ensure!(min_tokens > Zero::zero(), Error::<T>::MinTokensIsZero);
            let mut exchange = Self::get_exchange(&asset_id)?;
            match T::Assets::can_withdraw(
                exchange.liquidity_token_id.clone(),
                &caller,
                liquidity_amount,
            ) {
                WithdrawConsequence::Success => (),
                WithdrawConsequence::UnknownAsset => Err(Error::<T>::AssetNotFound)?,
                _ => Err(Error::<T>::ProviderLiquidityTooLow)?,
            };

            // --------------- Withdrawn currency/tokens computation ---------------
            let currency_reserve = T::CurrencyToAssetBalance::convert(exchange.currency_reserve);
            let total_liquidity = T::Assets::total_issuance(exchange.liquidity_token_id.clone());
            let currency_amount =
                FixedU128::saturating_from_rational(liquidity_amount, total_liquidity)
                    .saturating_mul_int(currency_reserve);
            let currency_amount = T::AssetToCurrencyBalance::convert(currency_amount);
            let token_amount =
                FixedU128::saturating_from_rational(liquidity_amount, total_liquidity)
                    .saturating_mul_int(exchange.token_reserve);
            ensure!(
                currency_amount >= min_currency,
                Error::<T>::MinCurrencyTooHigh
            );
            ensure!(token_amount >= min_tokens, Error::<T>::MinTokensTooHigh);

            // --------------------- Currency & token transfer ---------------------
            T::AssetRegistry::burn_from(
                exchange.liquidity_token_id.clone(),
                &caller,
                liquidity_amount,
            )?;
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
            exchange.currency_reserve.saturating_reduce(currency_amount);
            exchange.token_reserve.saturating_reduce(token_amount);
            <Exchanges<T>>::insert(asset_id.clone(), exchange);

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

        /// Exchange currency for asset.
        ///
        /// User specifies exact input (`currency_amount`) and minimum output (`min_tokens`).
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn currency_to_asset_swap_input(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            currency_amount: BalanceOf<T>,
            min_tokens: AssetBalanceOf<T>,
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::currency_to_asset_input(
                asset_id,
                currency_amount,
                min_tokens,
                deadline,
                caller.clone(),
                caller,
            )
        }

        /// Exchange currency for asset and transfer asset to recipient.
        ///
        /// User specifies exact input (`currency_amount`) and minimum output (`min_tokens`).
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn currency_to_asset_transfer_input(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            currency_amount: BalanceOf<T>,
            min_tokens: AssetBalanceOf<T>,
            deadline: T::BlockNumber,
            recipient: AccountIdOf<T>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::currency_to_asset_input(
                asset_id,
                currency_amount,
                min_tokens,
                deadline,
                caller,
                recipient,
            )
        }

        /// Exchange currency for asset.
        ///
        /// User specifies exact output (`token_amount`) and minimum input (`max_currency`).
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn currency_to_asset_swap_output(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            max_currency: BalanceOf<T>,
            token_amount: AssetBalanceOf<T>,
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::currency_to_asset_output(
                asset_id,
                max_currency,
                token_amount,
                deadline,
                caller.clone(),
                caller,
            )
        }

        /// Exchange currency for asset and transfer asset to recipient.
        ///
        /// User specifies exact output (`token_amount`) and minimum input (`max_currency`).
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn currency_to_asset_transfer_output(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            max_currency: BalanceOf<T>,
            token_amount: AssetBalanceOf<T>,
            deadline: T::BlockNumber,
            recipient: AccountIdOf<T>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::currency_to_asset_output(
                asset_id,
                max_currency,
                token_amount,
                deadline,
                caller,
                recipient,
            )
        }

        /// Exchange asset for currency.
        ///
        /// User specifies exact input (`token_amount`) and minimum output (`min_currency`).
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn asset_to_currency_swap_input(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            min_currency: BalanceOf<T>,
            token_amount: AssetBalanceOf<T>,
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::asset_to_currency_input(
                asset_id,
                min_currency,
                token_amount,
                deadline,
                caller.clone(),
                caller,
            )
        }

        /// Exchange asset for currency and transfer currency to recipient.
        ///
        /// User specifies exact input (`token_amount`) and minimum output (`min_currency`).
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn asset_to_currency_transfer_input(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            min_currency: BalanceOf<T>,
            token_amount: AssetBalanceOf<T>,
            deadline: T::BlockNumber,
            recipient: AccountIdOf<T>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::asset_to_currency_input(
                asset_id,
                min_currency,
                token_amount,
                deadline,
                caller,
                recipient,
            )
        }
    }

    impl<T: Config> Pallet<T> {
        fn get_exchange(asset_id: &AssetIdOf<T>) -> Result<ExchangeOf<T>, Error<T>> {
            <Exchanges<T>>::get(asset_id.clone()).ok_or(Error::<T>::ExchangeNotFound)
        }

        fn check_deadline(deadline: &T::BlockNumber) -> Result<(), Error<T>> {
            ensure!(
                deadline >= &<frame_system::Pallet<T>>::block_number(),
                Error::DeadlinePassed
            );
            Ok(())
        }

        fn get_input_price(
            input_amount: &BalanceOf<T>,
            input_reserve: &BalanceOf<T>,
            output_reserve: &BalanceOf<T>,
        ) -> Result<BalanceOf<T>, Error<T>> {
            debug_assert!(!input_reserve.is_zero());
            debug_assert!(!output_reserve.is_zero());
            debug_assert!(input_amount < input_reserve);
            let net_amount_numerator = T::ProviderFeeDenominator::get()
                .checked_sub(&T::ProviderFeeNumerator::get())
                .ok_or(Error::Underflow)?;
            let input_amount_with_fee = input_amount
                .checked_mul(&net_amount_numerator)
                .ok_or(Error::Overflow)?;
            let numerator = input_amount_with_fee
                .checked_mul(output_reserve)
                .ok_or(Error::Overflow)?;
            let denominator = input_reserve
                .checked_mul(&T::ProviderFeeDenominator::get())
                .ok_or(Error::Overflow)?
                .checked_add(&input_amount_with_fee)
                .ok_or(Error::Overflow)?;
            Ok(numerator / denominator)
        }

        fn get_output_price(
            output_amount: &BalanceOf<T>,
            input_reserve: &BalanceOf<T>,
            output_reserve: &BalanceOf<T>,
        ) -> Result<BalanceOf<T>, Error<T>> {
            debug_assert!(!input_reserve.is_zero());
            debug_assert!(!output_reserve.is_zero());
            debug_assert!(output_amount < output_reserve);
            let net_amount_numerator = T::ProviderFeeDenominator::get()
                .checked_sub(&T::ProviderFeeNumerator::get())
                .ok_or(Error::Underflow)?;
            let numerator = input_reserve
                .checked_mul(output_amount)
                .ok_or(Error::Overflow)?
                .checked_mul(&T::ProviderFeeDenominator::get())
                .ok_or(Error::Overflow)?;
            let denominator = output_reserve
                .saturating_sub(*output_amount)
                .checked_mul(&net_amount_numerator)
                .ok_or(Error::Overflow)?;
            Ok((numerator / denominator).saturating_add(<BalanceOf<T>>::one()))
        }

        /// Perform currency and asset transfers, update exchange balances, emit event
        #[transactional]
        fn swap_currency_for_asset(
            mut exchange: ExchangeOf<T>,
            currency_amount: BalanceOf<T>,
            token_amount: AssetBalanceOf<T>,
            buyer: AccountIdOf<T>,
            recipient: AccountIdOf<T>,
        ) -> DispatchResult {
            // --------------------- Currency & token transfer ---------------------
            let asset_id = exchange.asset_id.clone();
            let pallet_account = T::PalletId::get().into_account_truncating();
            <T as pallet::Config>::Currency::transfer(
                &buyer,
                &pallet_account,
                currency_amount,
                ExistenceRequirement::AllowDeath,
            )?;
            T::Assets::transfer(
                asset_id.clone(),
                &pallet_account,
                &recipient,
                token_amount,
                false,
            )?;

            // -------------------------- Balances update --------------------------
            exchange.currency_reserve.saturating_accrue(currency_amount);
            exchange.token_reserve.saturating_reduce(token_amount);
            <Exchanges<T>>::insert(asset_id.clone(), exchange);

            // ---------------------------- Emit event -----------------------------
            Self::deposit_event(Event::CurrencyTradedForAsset(
                asset_id,
                buyer,
                recipient,
                currency_amount,
                token_amount,
            ));
            Ok(())
        }

        /// Perform currency and asset transfers, update exchange balances, emit event
        #[transactional]
        fn swap_asset_for_currency(
            mut exchange: ExchangeOf<T>,
            currency_amount: BalanceOf<T>,
            token_amount: AssetBalanceOf<T>,
            buyer: AccountIdOf<T>,
            recipient: AccountIdOf<T>,
        ) -> DispatchResult {
            // --------------------- Currency & token transfer ---------------------
            let asset_id = exchange.asset_id.clone();
            let pallet_account = T::PalletId::get().into_account_truncating();
            T::Assets::transfer(
                asset_id.clone(),
                &buyer,
                &pallet_account,
                token_amount,
                false,
            )?;
            <T as pallet::Config>::Currency::transfer(
                &pallet_account,
                &recipient,
                currency_amount,
                ExistenceRequirement::AllowDeath,
            )?;

            // -------------------------- Balances update --------------------------
            exchange.token_reserve.saturating_accrue(token_amount);
            exchange.currency_reserve.saturating_reduce(currency_amount);
            <Exchanges<T>>::insert(asset_id.clone(), exchange);

            // ---------------------------- Emit event -----------------------------
            Self::deposit_event(Event::AssetTradedForCurrency(
                asset_id,
                buyer,
                recipient,
                currency_amount,
                token_amount,
            ));
            Ok(())
        }

        fn currency_to_asset_input(
            asset_id: AssetIdOf<T>,
            currency_amount: BalanceOf<T>,
            min_tokens: AssetBalanceOf<T>,
            deadline: T::BlockNumber,
            buyer: AccountIdOf<T>,
            recipient: AccountIdOf<T>,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            Self::check_deadline(&deadline)?;
            ensure!(
                currency_amount > Zero::zero(),
                Error::<T>::CurrencyAmountIsZero
            );
            ensure!(min_tokens > Zero::zero(), Error::<T>::MinTokensIsZero);
            ensure!(
                <T as Config>::Currency::free_balance(&buyer) >= currency_amount,
                Error::<T>::BalanceTooLow
            );
            let mut exchange = Self::get_exchange(&asset_id)?;
            ensure!(
                min_tokens < exchange.token_reserve,
                Error::<T>::NotEnoughLiquidity
            );

            // ----------------------- Compute token amount ------------------------
            let token_amount = Self::get_input_price(
                &currency_amount,
                &exchange.currency_reserve,
                &T::AssetToCurrencyBalance::convert(exchange.token_reserve),
            )?;
            let token_amount = T::CurrencyToAssetBalance::convert(token_amount);
            ensure!(token_amount >= min_tokens, Error::<T>::MinTokensTooHigh);

            // ------------------------- Perform the trade -------------------------
            Self::swap_currency_for_asset(exchange, currency_amount, token_amount, buyer, recipient)
        }

        fn currency_to_asset_output(
            asset_id: AssetIdOf<T>,
            max_currency: BalanceOf<T>,
            token_amount: AssetBalanceOf<T>,
            deadline: T::BlockNumber,
            buyer: AccountIdOf<T>,
            recipient: AccountIdOf<T>,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            Self::check_deadline(&deadline)?;
            ensure!(max_currency > Zero::zero(), Error::<T>::MaxCurrencyIsZero);
            ensure!(token_amount > Zero::zero(), Error::<T>::TokenAmountIsZero);
            let mut exchange = Self::get_exchange(&asset_id)?;
            ensure!(
                token_amount < exchange.token_reserve,
                Error::<T>::NotEnoughLiquidity
            );

            // ---------------------- Compute currency amount ----------------------
            let currency_amount = Self::get_output_price(
                &T::AssetToCurrencyBalance::convert(token_amount),
                &exchange.currency_reserve,
                &T::AssetToCurrencyBalance::convert(exchange.token_reserve),
            )?;
            ensure!(
                currency_amount <= max_currency,
                Error::<T>::MaxCurrencyTooLow
            );

            // ------------------------- Perform the trade -------------------------
            Self::swap_currency_for_asset(exchange, currency_amount, token_amount, buyer, recipient)
        }

        fn asset_to_currency_input(
            asset_id: AssetIdOf<T>,
            min_currency: BalanceOf<T>,
            token_amount: AssetBalanceOf<T>,
            deadline: T::BlockNumber,
            buyer: AccountIdOf<T>,
            recipient: AccountIdOf<T>,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            Self::check_deadline(&deadline)?;
            ensure!(min_currency > Zero::zero(), Error::<T>::MinCurrencyIsZero);
            ensure!(token_amount > Zero::zero(), Error::<T>::TokenAmountIsZero);
            match T::Assets::can_withdraw(asset_id.clone(), &buyer, token_amount) {
                WithdrawConsequence::Success => (),
                WithdrawConsequence::UnknownAsset => Err(Error::<T>::AssetNotFound)?,
                _ => Err(Error::<T>::NotEnoughTokens)?,
            };
            let mut exchange = Self::get_exchange(&asset_id)?;
            ensure!(
                min_currency < exchange.currency_reserve,
                Error::<T>::NotEnoughLiquidity
            );

            // ---------------------- Compute currency amount ----------------------
            let currency_amount = Self::get_input_price(
                &T::AssetToCurrencyBalance::convert(token_amount),
                &T::AssetToCurrencyBalance::convert(exchange.token_reserve),
                &exchange.currency_reserve,
            )?;
            ensure!(
                currency_amount >= min_currency,
                Error::<T>::MinCurrencyTooHigh
            );

            // ------------------------- Perform the trade -------------------------
            Self::swap_asset_for_currency(exchange, currency_amount, token_amount, buyer, recipient)
        }
    }
}
