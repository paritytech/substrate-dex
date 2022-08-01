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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
pub mod rpc;
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
    use frame_support::{
        pallet_prelude::*,
        sp_runtime::{
            traits::{
                AccountIdConversion, CheckedAdd, CheckedMul, CheckedSub, Convert, One, Saturating,
                Zero,
            },
            FixedPointNumber, FixedPointOperand, FixedU128,
        },
        traits::{
            fungibles::{Create, Destroy, Inspect, Mutate, Transfer},
            tokens::{Balance, WithdrawConsequence},
            ExistenceRequirement, Randomness,
        },
        transactional, PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_std::fmt::Debug;

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

    pub trait ConfigHelper: Config {
        fn pallet_account() -> AccountIdOf<Self>;
        fn currency_to_asset(curr_balance: BalanceOf<Self>) -> AssetBalanceOf<Self>;
        fn asset_to_currency(asset_balance: AssetBalanceOf<Self>) -> BalanceOf<Self>;
        fn net_amount_numerator() -> BalanceOf<Self>;
    }

    impl<T: Config> ConfigHelper for T {
        #[inline(always)]
        fn pallet_account() -> AccountIdOf<Self> {
            Self::PalletId::get().into_account_truncating()
        }

        #[inline(always)]
        fn currency_to_asset(curr_balance: BalanceOf<Self>) -> AssetBalanceOf<Self> {
            Self::CurrencyToAssetBalance::convert(curr_balance)
        }

        #[inline(always)]
        fn asset_to_currency(asset_balance: AssetBalanceOf<Self>) -> BalanceOf<Self> {
            Self::AssetToCurrencyBalance::convert(asset_balance)
        }

        #[inline(always)]
        fn net_amount_numerator() -> BalanceOf<Self> {
            Self::ProviderFeeDenominator::get()
                .checked_sub(&Self::ProviderFeeNumerator::get())
                .expect("Provider fee shouldn't be greater than 100%")
        }
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
        /// Specified account doesn't own enough liquidity in the exchange
        ProviderLiquidityTooLow,
        /// No exchange found for the given `asset_id`
        ExchangeNotFound,
        /// Zero value provided for trade amount parameter
        TradeAmountIsZero,
        /// Zero value provided for `max_tokens` parameter
        MaxTokensIsZero,
        /// Zero value provided for `currency_amount` parameter
        CurrencyAmountIsZero,
        /// Value provided for `currency_amount` parameter is too high
        CurrencyAmountTooHigh,
        /// Zero value provided for `min_liquidity` parameter
        MinLiquidityIsZero,
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
        /// Value provided for `min_currency` parameter is too high
        MinCurrencyTooHigh,
        /// Value provided for `min_tokens` parameter is too high
        MinTokensTooHigh,
        /// Value provided for `max_currency` parameter is too low
        MaxCurrencyTooLow,
        /// Value provided for `min_bought_tokens` parameter is too high
        MinBoughtTokensTooHigh,
        /// Value provided for `max_sold_tokens` parameter is too low
        MaxSoldTokensTooLow,
        // /// There is not enough liquidity in the exchange to perform trade
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

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    pub enum TradeAmount<InputBalance, OutputBalance> {
        FixedInput {
            input_amount: InputBalance,
            min_output: OutputBalance,
        },
        FixedOutput {
            max_input: InputBalance,
            output_amount: OutputBalance,
        },
    }

    // (sold_token_amount, currency_amount, bought_token_amount)
    type AssetToAssetPrice<T> = (AssetBalanceOf<T>, BalanceOf<T>, AssetBalanceOf<T>);

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
            let _caller = ensure_signed(origin)?;
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
            T::AssetRegistry::create(
                liquidity_token_id.clone(),
                T::pallet_account(),
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
            ensure!(currency_amount > Zero::zero(), Error::<T>::CurrencyAmountIsZero);
            ensure!(max_tokens > Zero::zero(), Error::<T>::MaxTokensIsZero);
            Self::check_enough_currency(&caller, &currency_amount)?;
            Self::check_enough_tokens(&asset_id, &caller, &max_tokens)?;
            let exchange = Self::get_exchange(&asset_id)?;

            // -------------------- Token/liquidity computation --------------------
            let total_liquidity = T::Assets::total_issuance(exchange.liquidity_token_id.clone());
            let (token_amount, liquidity_minted) = if total_liquidity > Zero::zero() {
                ensure!(min_liquidity > Zero::zero(), Error::<T>::MinLiquidityIsZero);
                let currency_amount = T::currency_to_asset(currency_amount);
                let currency_reserve = T::currency_to_asset(exchange.currency_reserve);
                let token_amount =
                    FixedU128::saturating_from_rational(currency_amount, currency_reserve)
                        .saturating_mul_int(exchange.token_reserve)
                        .saturating_add(One::one());
                let liquidity_minted =
                    FixedU128::saturating_from_rational(currency_amount, currency_reserve)
                        .saturating_mul_int(total_liquidity);
                ensure!(token_amount <= max_tokens, Error::<T>::MaxTokensTooLow);
                ensure!(liquidity_minted >= min_liquidity, Error::<T>::MinLiquidityTooHigh);
                (token_amount, liquidity_minted)
            } else {
                (max_tokens, T::currency_to_asset(currency_amount))
            };

            // ----------------------------- State update ----------------------------
            Self::do_add_liquidity(
                exchange,
                currency_amount,
                token_amount,
                liquidity_minted,
                caller,
            )
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
            ensure!(liquidity_amount > Zero::zero(), Error::<T>::LiquidityAmountIsZero);
            ensure!(min_currency > Zero::zero(), Error::<T>::MinCurrencyIsZero);
            ensure!(min_tokens > Zero::zero(), Error::<T>::MinTokensIsZero);
            let exchange = Self::get_exchange(&asset_id)?;
            Self::check_enough_liquidity_owned(&exchange, &caller, &liquidity_amount)?;

            // --------------- Withdrawn currency/tokens computation ---------------
            let currency_reserve = T::currency_to_asset(exchange.currency_reserve);
            let total_liquidity = T::Assets::total_issuance(exchange.liquidity_token_id.clone());
            let currency_amount =
                FixedU128::saturating_from_rational(liquidity_amount, total_liquidity)
                    .saturating_mul_int(currency_reserve);
            let currency_amount = T::asset_to_currency(currency_amount);
            let token_amount =
                FixedU128::saturating_from_rational(liquidity_amount, total_liquidity)
                    .saturating_mul_int(exchange.token_reserve);
            ensure!(currency_amount >= min_currency, Error::<T>::MinCurrencyTooHigh);
            ensure!(token_amount >= min_tokens, Error::<T>::MinTokensTooHigh);

            // ----------------------------- State update ----------------------------
            Self::do_remove_liquidity(
                exchange,
                currency_amount,
                token_amount,
                liquidity_amount,
                caller,
            )
        }

        /// Exchange currency for asset. Optionally, transfer asset to `recipient`.
        ///
        /// User can specify either:
        ///   - exact input (`input_amount`) and minimum output (`min_output`), or
        ///   - exact output (`output_amount`) and maximum input (`max_input`).
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn currency_to_asset(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            amount: TradeAmount<BalanceOf<T>, AssetBalanceOf<T>>,
            deadline: T::BlockNumber,
            recipient: Option<AccountIdOf<T>>,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            let caller = ensure_signed(origin)?;
            let recipient = recipient.unwrap_or_else(|| caller.clone());
            Self::check_deadline(&deadline)?;
            Self::check_trade_amount(&amount)?;
            let exchange = Self::get_exchange(&asset_id)?;

            // --------------------------- Compute price ---------------------------
            let (currency_amount, token_amount) =
                Self::get_currency_to_asset_price(&exchange, amount)?;
            Self::check_enough_currency(&caller, &currency_amount)?;

            // --------------------------- Perform trade ---------------------------
            Self::swap_currency_for_asset(
                exchange,
                currency_amount,
                token_amount,
                caller,
                recipient,
            )
        }

        /// Exchange asset for currency. Optionally, transfer currency to `recipient`.
        ///
        /// User can specify either:
        ///   - exact input (`input_amount`) and minimum output (`min_output`), or
        ///   - exact output (`output_amount`) and maximum input (`max_input`).
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn asset_to_currency(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            amount: TradeAmount<AssetBalanceOf<T>, BalanceOf<T>>,
            deadline: T::BlockNumber,
            recipient: Option<AccountIdOf<T>>,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            let caller = ensure_signed(origin)?;
            let recipient = recipient.unwrap_or_else(|| caller.clone());
            Self::check_deadline(&deadline)?;
            Self::check_trade_amount(&amount)?;
            let exchange = Self::get_exchange(&asset_id)?;

            // --------------------------- Compute price ---------------------------
            let (currency_amount, token_amount) =
                Self::get_asset_to_currency_price(&exchange, amount)?;
            Self::check_enough_tokens(&asset_id, &caller, &token_amount)?;

            // --------------------------- Perform trade ---------------------------
            Self::swap_asset_for_currency(
                exchange,
                currency_amount,
                token_amount,
                caller,
                recipient,
            )
        }

        /// Exchange asset for another asset. Optionally, transfer asset to `recipient`.

        /// User can specify either:
        ///   - exact input (`input_amount`) and minimum output (`min_output`), or
        ///   - exact output (`output_amount`) and maximum input (`max_input`).
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(1000)]
        pub fn asset_to_asset(
            origin: OriginFor<T>,
            sold_asset_id: AssetIdOf<T>,
            bought_asset_id: AssetIdOf<T>,
            amount: TradeAmount<AssetBalanceOf<T>, AssetBalanceOf<T>>,
            deadline: T::BlockNumber,
            recipient: Option<AccountIdOf<T>>,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            let caller = ensure_signed(origin)?;
            let recipient = recipient.unwrap_or_else(|| caller.clone());
            Self::check_deadline(&deadline)?;
            Self::check_trade_amount(&amount)?;
            let sold_asset_exchange = Self::get_exchange(&sold_asset_id)?;
            let bought_asset_exchange = Self::get_exchange(&bought_asset_id)?;

            // --------------------------- Compute price ---------------------------
            let (sold_token_amount, currency_amount, bought_token_amount) =
                Self::get_asset_to_asset_price(
                    &sold_asset_exchange,
                    &bought_asset_exchange,
                    amount,
                )?;
            Self::check_enough_tokens(&sold_asset_id, &caller, &sold_token_amount)?;

            // --------------------------- Perform trade ---------------------------
            Self::swap_asset_for_asset(
                sold_asset_exchange,
                bought_asset_exchange,
                currency_amount,
                sold_token_amount,
                bought_token_amount,
                caller,
                recipient,
            )
        }
    }

    impl<T: Config> Pallet<T> {
        pub(crate) fn get_exchange(asset_id: &AssetIdOf<T>) -> Result<ExchangeOf<T>, Error<T>> {
            <Exchanges<T>>::get(asset_id.clone()).ok_or(Error::<T>::ExchangeNotFound)
        }

        fn check_deadline(deadline: &T::BlockNumber) -> Result<(), Error<T>> {
            ensure!(deadline >= &<frame_system::Pallet<T>>::block_number(), Error::DeadlinePassed);
            Ok(())
        }

        fn check_trade_amount<A: Zero, B: Zero>(
            amount: &TradeAmount<A, B>,
        ) -> Result<(), Error<T>> {
            match amount {
                TradeAmount::FixedInput {
                    input_amount,
                    min_output,
                } => {
                    ensure!(!input_amount.is_zero(), Error::TradeAmountIsZero);
                    ensure!(!min_output.is_zero(), Error::TradeAmountIsZero);
                }
                TradeAmount::FixedOutput {
                    output_amount,
                    max_input,
                } => {
                    ensure!(!output_amount.is_zero(), Error::TradeAmountIsZero);
                    ensure!(!max_input.is_zero(), Error::TradeAmountIsZero);
                }
            };
            Ok(())
        }

        fn check_enough_currency(
            account_id: &AccountIdOf<T>,
            amount: &BalanceOf<T>,
        ) -> Result<(), Error<T>> {
            ensure!(
                &<T as Config>::Currency::free_balance(account_id) >= amount,
                Error::<T>::BalanceTooLow
            );
            Ok(())
        }

        fn check_enough_tokens(
            asset_id: &AssetIdOf<T>,
            account_id: &AccountIdOf<T>,
            amount: &AssetBalanceOf<T>,
        ) -> Result<(), Error<T>> {
            match T::Assets::can_withdraw(asset_id.clone(), account_id, *amount) {
                WithdrawConsequence::Success => Ok(()),
                WithdrawConsequence::ReducedToZero(_) => Ok(()),
                WithdrawConsequence::UnknownAsset => Err(Error::<T>::AssetNotFound),
                _ => Err(Error::<T>::NotEnoughTokens),
            }
        }

        fn check_enough_liquidity_owned(
            exchange: &ExchangeOf<T>,
            account_id: &AccountIdOf<T>,
            amount: &AssetBalanceOf<T>,
        ) -> Result<(), Error<T>> {
            let asset_id = exchange.liquidity_token_id.clone();
            match T::AssetRegistry::can_withdraw(asset_id, account_id, *amount) {
                WithdrawConsequence::Success => Ok(()),
                WithdrawConsequence::ReducedToZero(_) => Ok(()),
                WithdrawConsequence::UnknownAsset => Err(Error::<T>::AssetNotFound),
                _ => Err(Error::<T>::ProviderLiquidityTooLow),
            }
        }

        pub(crate) fn get_input_price(
            input_amount: &BalanceOf<T>,
            input_reserve: &BalanceOf<T>,
            output_reserve: &BalanceOf<T>,
        ) -> Result<BalanceOf<T>, Error<T>> {
            debug_assert!(!input_reserve.is_zero());
            debug_assert!(!output_reserve.is_zero());
            let input_amount_with_fee = input_amount
                .checked_mul(&T::net_amount_numerator())
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

        pub(crate) fn get_output_price(
            output_amount: &BalanceOf<T>,
            input_reserve: &BalanceOf<T>,
            output_reserve: &BalanceOf<T>,
        ) -> Result<BalanceOf<T>, Error<T>> {
            debug_assert!(!input_reserve.is_zero());
            debug_assert!(!output_reserve.is_zero());
            ensure!(output_amount < output_reserve, Error::<T>::NotEnoughLiquidity);
            let numerator = input_reserve
                .checked_mul(output_amount)
                .ok_or(Error::Overflow)?
                .checked_mul(&T::ProviderFeeDenominator::get())
                .ok_or(Error::Overflow)?;
            let denominator = output_reserve
                .saturating_sub(*output_amount)
                .checked_mul(&T::net_amount_numerator())
                .ok_or(Error::Overflow)?;
            Ok((numerator / denominator).saturating_add(<BalanceOf<T>>::one()))
        }

        fn get_currency_to_asset_price(
            exchange: &ExchangeOf<T>,
            amount: TradeAmount<BalanceOf<T>, AssetBalanceOf<T>>,
        ) -> Result<(BalanceOf<T>, AssetBalanceOf<T>), Error<T>> {
            match amount {
                TradeAmount::FixedInput {
                    input_amount: currency_amount,
                    min_output: min_tokens,
                } => {
                    let token_amount = Self::get_input_price(
                        &currency_amount,
                        &exchange.currency_reserve,
                        &T::asset_to_currency(exchange.token_reserve),
                    )?;
                    let token_amount = T::currency_to_asset(token_amount);
                    ensure!(token_amount >= min_tokens, Error::MinTokensTooHigh);
                    Ok((currency_amount, token_amount))
                }
                TradeAmount::FixedOutput {
                    max_input: max_currency,
                    output_amount: token_amount,
                } => {
                    let currency_amount = Self::get_output_price(
                        &T::asset_to_currency(token_amount),
                        &exchange.currency_reserve,
                        &T::asset_to_currency(exchange.token_reserve),
                    )?;
                    ensure!(currency_amount <= max_currency, Error::MaxCurrencyTooLow);
                    Ok((currency_amount, token_amount))
                }
            }
        }

        fn get_asset_to_currency_price(
            exchange: &ExchangeOf<T>,
            amount: TradeAmount<AssetBalanceOf<T>, BalanceOf<T>>,
        ) -> Result<(BalanceOf<T>, AssetBalanceOf<T>), Error<T>> {
            match amount {
                TradeAmount::FixedInput {
                    input_amount: token_amount,
                    min_output: min_currency,
                } => {
                    let currency_amount = Self::get_input_price(
                        &T::asset_to_currency(token_amount),
                        &T::asset_to_currency(exchange.token_reserve),
                        &exchange.currency_reserve,
                    )?;
                    ensure!(currency_amount >= min_currency, Error::MinCurrencyTooHigh);
                    Ok((currency_amount, token_amount))
                }
                TradeAmount::FixedOutput {
                    max_input: max_tokens,
                    output_amount: currency_amount,
                } => {
                    let token_amount = Self::get_output_price(
                        &currency_amount,
                        &T::asset_to_currency(exchange.token_reserve),
                        &exchange.currency_reserve,
                    )?;
                    let token_amount = T::currency_to_asset(token_amount);
                    ensure!(token_amount <= max_tokens, Error::MaxTokensTooLow);
                    Ok((currency_amount, token_amount))
                }
            }
        }

        fn get_asset_to_asset_price(
            sold_asset_exchange: &ExchangeOf<T>,
            bought_asset_exchange: &ExchangeOf<T>,
            amount: TradeAmount<AssetBalanceOf<T>, AssetBalanceOf<T>>,
        ) -> Result<AssetToAssetPrice<T>, Error<T>> {
            match amount {
                TradeAmount::FixedInput {
                    input_amount: sold_token_amount,
                    min_output: min_bought_tokens,
                } => {
                    let currency_amount = Self::get_input_price(
                        &T::asset_to_currency(sold_token_amount),
                        &T::asset_to_currency(sold_asset_exchange.token_reserve),
                        &sold_asset_exchange.currency_reserve,
                    )?;
                    let bought_token_amount = Self::get_input_price(
                        &currency_amount,
                        &bought_asset_exchange.currency_reserve,
                        &T::asset_to_currency(bought_asset_exchange.token_reserve),
                    )?;
                    let bought_token_amount = T::currency_to_asset(bought_token_amount);
                    ensure!(
                        bought_token_amount >= min_bought_tokens,
                        Error::<T>::MinBoughtTokensTooHigh
                    );
                    Ok((sold_token_amount, currency_amount, bought_token_amount))
                }
                TradeAmount::FixedOutput {
                    max_input: max_sold_tokens,
                    output_amount: bought_token_amount,
                } => {
                    let currency_amount = Self::get_output_price(
                        &T::asset_to_currency(bought_token_amount),
                        &bought_asset_exchange.currency_reserve,
                        &T::asset_to_currency(bought_asset_exchange.token_reserve),
                    )?;
                    let sold_token_amount = Self::get_output_price(
                        &currency_amount,
                        &T::asset_to_currency(sold_asset_exchange.token_reserve),
                        &sold_asset_exchange.currency_reserve,
                    )?;
                    let sold_token_amount = T::currency_to_asset(sold_token_amount);
                    ensure!(sold_token_amount <= max_sold_tokens, Error::<T>::MaxSoldTokensTooLow);
                    Ok((sold_token_amount, currency_amount, bought_token_amount))
                }
            }
        }

        /// Perform currency and asset transfers, mint liquidity token,
        /// update exchange balances, emit event
        #[transactional]
        fn do_add_liquidity(
            mut exchange: ExchangeOf<T>,
            currency_amount: BalanceOf<T>,
            token_amount: AssetBalanceOf<T>,
            liquidity_minted: AssetBalanceOf<T>,
            provider: AccountIdOf<T>,
        ) -> DispatchResult {
            // --------------------- Currency & token transfer ---------------------
            let asset_id = exchange.asset_id.clone();
            let pallet_account = T::pallet_account();
            <T as pallet::Config>::Currency::transfer(
                &provider,
                &pallet_account,
                currency_amount,
                ExistenceRequirement::KeepAlive,
            )?;
            T::Assets::transfer(asset_id.clone(), &provider, &pallet_account, token_amount, true)?;
            T::AssetRegistry::mint_into(
                exchange.liquidity_token_id.clone(),
                &provider,
                liquidity_minted,
            )?;

            // -------------------------- Balances update --------------------------
            exchange.currency_reserve.saturating_accrue(currency_amount);
            exchange.token_reserve.saturating_accrue(token_amount);
            <Exchanges<T>>::insert(asset_id.clone(), exchange);

            // ---------------------------- Emit event -----------------------------
            Self::deposit_event(Event::LiquidityAdded(
                provider,
                asset_id,
                currency_amount,
                token_amount,
                liquidity_minted,
            ));
            Ok(())
        }

        /// Perform currency and asset transfers, burn liquidity token,
        /// update exchange balances, emit event
        #[transactional]
        fn do_remove_liquidity(
            mut exchange: ExchangeOf<T>,
            currency_amount: BalanceOf<T>,
            token_amount: AssetBalanceOf<T>,
            liquidity_amount: AssetBalanceOf<T>,
            provider: AccountIdOf<T>,
        ) -> DispatchResult {
            // --------------------- Currency & token transfer ---------------------
            let asset_id = exchange.asset_id.clone();
            let pallet_account = T::pallet_account();
            T::AssetRegistry::burn_from(
                exchange.liquidity_token_id.clone(),
                &provider,
                liquidity_amount,
            )?;
            <T as pallet::Config>::Currency::transfer(
                &pallet_account,
                &provider,
                currency_amount,
                ExistenceRequirement::AllowDeath,
            )?;
            T::Assets::transfer(asset_id.clone(), &pallet_account, &provider, token_amount, false)?;

            // -------------------------- Balances update --------------------------
            exchange.currency_reserve.saturating_reduce(currency_amount);
            exchange.token_reserve.saturating_reduce(token_amount);
            <Exchanges<T>>::insert(asset_id.clone(), exchange);

            // ---------------------------- Emit event -----------------------------
            Self::deposit_event(Event::LiquidityRemoved(
                provider,
                asset_id,
                currency_amount,
                token_amount,
                liquidity_amount,
            ));
            Ok(())
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
            let pallet_account = T::pallet_account();
            if buyer != pallet_account {
                <T as pallet::Config>::Currency::transfer(
                    &buyer,
                    &pallet_account,
                    currency_amount,
                    ExistenceRequirement::AllowDeath,
                )?;
            }
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
            let pallet_account = T::pallet_account();
            T::Assets::transfer(asset_id.clone(), &buyer, &pallet_account, token_amount, false)?;
            if recipient != pallet_account {
                <T as pallet::Config>::Currency::transfer(
                    &pallet_account,
                    &recipient,
                    currency_amount,
                    ExistenceRequirement::AllowDeath,
                )?;
            }

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

        /// Swap one asset to currency, then currency to another asset
        #[transactional]
        fn swap_asset_for_asset(
            sold_asset_exchange: ExchangeOf<T>,
            bought_asset_exchange: ExchangeOf<T>,
            currency_amount: BalanceOf<T>,
            sold_token_amount: AssetBalanceOf<T>,
            bought_token_amount: AssetBalanceOf<T>,
            buyer: AccountIdOf<T>,
            recipient: AccountIdOf<T>,
        ) -> DispatchResult {
            let pallet_account: AccountIdOf<T> = T::pallet_account();
            Self::swap_asset_for_currency(
                sold_asset_exchange,
                currency_amount,
                sold_token_amount,
                buyer,
                pallet_account.clone(),
            )?;
            Self::swap_currency_for_asset(
                bought_asset_exchange,
                currency_amount,
                bought_token_amount,
                pallet_account,
                recipient,
            )
        }
    }
}
