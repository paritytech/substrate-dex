//! # DEX pallet
//!
//! ## Overview
//!
//! This pallet re-implements Uniswap V1 protocol for decentralized exchange of fungible assets.
//! Please refer to the [protocol description](https://docs.uniswap.org/protocol/V1/introduction)
//! and [smart contracts](https://github.com/Uniswap/v1-contracts) for more details.
//! DEX pallet allows users to create exchanges (i.e. liquidity pools), supply them with liquidity
//! (i.e. currency & assets), and perform trades (currency-to-asset, asset-to-currency, asset-to-asset).
//! DEX pallet also allows querying asset prices by custom RPC methods.
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
            ExistenceRequirement,
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
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The currency trait.
        type Currency: Currency<Self::AccountId>;

        /// The balance type for assets (i.e. tokens).
        type AssetBalance: Balance
            + FixedPointOperand
            + MaxEncodedLen
            + MaybeSerializeDeserialize
            + TypeInfo;

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

        /// Information on runtime weights.
        type WeightInfo: WeightInfo;

        /// Provider fee numerator.
        #[pallet::constant]
        type ProviderFeeNumerator: Get<BalanceOf<Self>>;

        /// Provider fee denominator.
        #[pallet::constant]
        type ProviderFeeDenominator: Get<BalanceOf<Self>>;

        /// Minimum currency deposit for a new exchange.
        #[pallet::constant]
        type MinDeposit: Get<BalanceOf<Self>>;
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

    type GenesisExchangeInfo<T> =
        (AccountIdOf<T>, AssetIdOf<T>, AssetIdOf<T>, BalanceOf<T>, AssetBalanceOf<T>);

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub exchanges: Vec<GenesisExchangeInfo<T>>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> GenesisConfig<T> {
            GenesisConfig { exchanges: vec![] }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let pallet_account = T::pallet_account();
            for (provider, asset_id, liquidity_token_id, currency_amount, token_amount) in
                &self.exchanges
            {
                // ----------------------- Create liquidity token ----------------------
                assert!(!<Exchanges<T>>::contains_key(asset_id), "Exchange already created");
                assert!(
                    T::AssetRegistry::create(
                        liquidity_token_id.clone(),
                        pallet_account.clone(),
                        false,
                        <AssetBalanceOf<T>>::one(),
                    )
                    .is_ok(),
                    "Liquidity token id already in use"
                );

                // -------------------------- Update storage ---------------------------
                let mut exchange = Exchange {
                    asset_id: asset_id.clone(),
                    currency_reserve: <BalanceOf<T>>::zero(),
                    token_reserve: <AssetBalanceOf<T>>::zero(),
                    liquidity_token_id: liquidity_token_id.clone(),
                };

                let liquidity_minted = T::currency_to_asset(*currency_amount);

                // --------------------- Currency & token transfer ---------------------
                assert!(
                    <T as pallet::Config>::Currency::transfer(
                        provider,
                        &pallet_account,
                        *currency_amount,
                        ExistenceRequirement::KeepAlive,
                    )
                    .is_ok(),
                    "Provider does not have enough amount of currency"
                );

                assert!(
                    T::Assets::transfer(
                        asset_id.clone(),
                        provider,
                        &pallet_account,
                        *token_amount,
                        true,
                    )
                    .is_ok(),
                    "Provider does not have enough amount of asset tokens"
                );

                assert!(
                    T::AssetRegistry::mint_into(
                        liquidity_token_id.clone(),
                        provider,
                        liquidity_minted
                    )
                    .is_ok(),
                    "Unexpected error while minting liquidity tokens for Provider"
                );

                // -------------------------- Balances update --------------------------
                exchange
                    .currency_reserve
                    .saturating_accrue(*currency_amount);
                exchange.token_reserve.saturating_accrue(*token_amount);
                <Exchanges<T>>::insert(asset_id.clone(), exchange);
            }
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
        /// Currency was traded for an asset [asset_id, buyer_id, recipient_id, currency_amount, token_amount]
        CurrencyTradedForAsset(
            AssetIdOf<T>,
            T::AccountId,
            T::AccountId,
            BalanceOf<T>,
            AssetBalanceOf<T>,
        ),
        /// An asset was traded for currency [asset_id, buyer_id, recipient_id, currency_amount, token_amount]
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
        /// Provided liquidity token ID is already taken
        TokenIdTaken,
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
        /// Zero value provided for `token_amount` parameter
        TokenAmountIsZero,
        /// Zero value provided for `max_tokens` parameter
        MaxTokensIsZero,
        /// Zero value provided for `currency_amount` parameter
        CurrencyAmountIsZero,
        /// Value provided for `currency_amount` parameter is too high
        CurrencyAmountTooHigh,
        /// Value provided for `currency_amount` parameter is too low
        CurrencyAmountTooLow,
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
        /// Create a new exchange. Deposit initial liquidity (currency & assets).
        /// Create a new liquidity token. Mint & transfer to the caller account an amount
        /// of the liquidity token equal to `currency_amount`.
        /// Emit two events on success: `ExchangeCreated` and `LiquidityAdded`.
        ///
        /// **Parameters:**
        ///   * `origin` – Origin for the call. Must be signed.
        ///   * `asset_id` – ID of the asset traded on the created exchange. Asset with this ID must exist.
        ///   * `liquidity_token_id` – ID of the liquidity token to be created. Asset with this ID must *not* exist.
        ///   * `currency_amount` – Initial amount of the currency to deposit in the pool. Must be at least equal `MinDeposit`.
        ///   * `token_amount` – Initial amount of tokens to deposit in the pool. Must be greater than 0.
        ///
        /// **Errors:**
        ///   * `AssetNotFound` – Asset with the given `asset_id` does not exist or has total supply equal 0.
        ///   * `ExchangeAlreadyExists` – An exchange fot the specified asset already exists.
        ///   * `TokenIdTaken` – Specified `liquidity_token_id` is already taken by another liquidity token.
        ///   * `CurrencyAmountTooLow` – Specified `currency_amount` is lower than `MinDeposit`.
        ///   * `TokenAmountIsZero` – Specified `token_amount` equals 0.
        #[pallet::weight(<T as Config>::WeightInfo::create_exchange())]
        #[transactional]
        pub fn create_exchange(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            liquidity_token_id: AssetIdOf<T>,
            currency_amount: BalanceOf<T>,
            token_amount: AssetBalanceOf<T>,
        ) -> DispatchResult {
            // -------------------------- Validation part --------------------------
            let caller = ensure_signed(origin)?;
            ensure!(currency_amount >= T::MinDeposit::get(), Error::<T>::CurrencyAmountTooLow);
            ensure!(token_amount > Zero::zero(), Error::<T>::TokenAmountIsZero);
            if T::Assets::total_issuance(asset_id.clone()).is_zero() {
                Err(Error::<T>::AssetNotFound)?
            }
            if <Exchanges<T>>::contains_key(asset_id.clone()) {
                Err(Error::<T>::ExchangeAlreadyExists)?
            }

            // ----------------------- Create liquidity token ----------------------
            T::AssetRegistry::create(
                liquidity_token_id.clone(),
                T::pallet_account(),
                false,
                <AssetBalanceOf<T>>::one(),
            )
            .map_err(|_| Error::<T>::TokenIdTaken)?;

            // -------------------------- Update storage ---------------------------
            let exchange = Exchange {
                asset_id: asset_id.clone(),
                currency_reserve: <BalanceOf<T>>::zero(),
                token_reserve: <AssetBalanceOf<T>>::zero(),
                liquidity_token_id: liquidity_token_id.clone(),
            };
            let liquidity_minted = T::currency_to_asset(currency_amount);
            Self::do_add_liquidity(
                exchange,
                currency_amount,
                token_amount,
                liquidity_minted,
                caller,
            )?;

            // ---------------------------- Emit event -----------------------------
            Self::deposit_event(Event::ExchangeCreated(asset_id, liquidity_token_id));
            Ok(())
        }

        /// Add liquidity to an existing exchange. The caller specifies an exact amount of currency
        /// to be deposited, a maximum amount of tokens to be deposited, and a minimum amount
        /// of liquidity tokens to receive. Emit `LiquidityAdded` event on success.
        ///
        /// **Parameters:**
        ///   * `origin` – Origin for the call. Must be signed.
        ///   * `asset_id` – ID of the deposited asset. An exchange for this asset must exist.
        ///   * `currency_amount` – The amount of the currency to deposit in the pool. Must be greater than 0.
        ///   * `min_liquidity` – The minimum amount of liquidity tokens to receive. Must be greater than 0.
        ///   * `max_tokens` – The maximum amount of tokens to be deposited. Must be greater than 0.
        ///   * `deadline` – Number of the last block in which the transaction can be included.
        ///
        /// **Errors:**
        ///   * `DeadlinePassed` – Specified `deadline` is lower than the current block number.
        ///   * `ExchangeNotFound` – There is no exchange for the given `asset_id`.
        ///   * `CurrencyAmountIsZero` – Specified `currency_amount` equals 0.
        ///   * `MinLiquidityIsZero` – Specified `min_liquidity` equals 0.
        ///   * `MaxTokensIsZero` – Specified `max_tokens` equals 0.
        ///   * `BalanceTooLow` – Specified `currency_amount` is greater than the available currency balance of the caller account.
        ///   * `NotEnoughTokens` – Specified `max_tokens` is greater than the available asset balance of the caller account.
        ///   * `MaxTokensTooLow` – Specified `max_tokens` is too low to match the `currency_amount`.
        ///     Currency and tokens need to be added proportionally.
        ///   * `MinLiquidityTooHigh` – The amount of liquidity tokes which would be minted by depositing the specified
        ///     `currency_amount` is lower than the specified `min_liquidity`.
        #[pallet::weight(<T as Config>::WeightInfo::add_liquidity())]
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
            ensure!(min_liquidity > Zero::zero(), Error::<T>::MinLiquidityIsZero);
            Self::check_enough_currency(&caller, &currency_amount)?;
            Self::check_enough_tokens(&asset_id, &caller, &max_tokens)?;
            let exchange = Self::get_exchange(&asset_id)?;

            // -------------------- Token/liquidity computation --------------------
            let total_liquidity = T::Assets::total_issuance(exchange.liquidity_token_id.clone());
            debug_assert!(total_liquidity > Zero::zero());
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

            // ----------------------------- State update ----------------------------
            Self::do_add_liquidity(
                exchange,
                T::asset_to_currency(currency_amount),
                token_amount,
                liquidity_minted,
                caller,
            )
        }

        /// Remove liquidity from an exchange. The caller specifies the amount of liquidity tokens
        /// to burn, and minimum amounts of currency and asset to receive.
        /// Emit `LiquidityRemoved` event on success.
        ///
        /// **Parameters:**
        ///   * `origin` – Origin for the call. Must be signed.
        ///   * `asset_id` – ID of the withdrawn asset. An exchange for this asset must exist.
        ///   * `liquidity_amount` – The amount of liquidity tokens to be burned. Must be greater than 0.
        ///   * `min_currency` – The minimum amount of currency to receive. Must be greater than 0.
        ///   * `min_tokens` – The minimum amount of tokens to receive. Must be greater than 0.
        ///   * `deadline` – Number of the last block in which the transaction can be included.
        ///
        /// **Errors:**
        ///   * `DeadlinePassed` – Specified `deadline` is lower than the current block number.
        ///   * `ExchangeNotFound` – There is no exchange for the given `asset_id`.
        ///   * `LiquidityAmountIsZero` – Specified `liquidity_amount` equals 0.
        ///   * `MinCurrencyIsZero` – Specified `min_currency` equals 0.
        ///   * `MinTokensIsZero` – Specified `min_tokens` equals 0.
        ///   * `ProviderLiquidityTooLow` – Specified `liquidity_amount` is greater than the liquidity
        ///     token balance of the caller account.
        ///   * `MinCurrencyTooHigh` – The amount of currency which could be received in exchange for the specified
        ///     `liquidity_amount` is lower than the specified `min_currency`.
        ///   * `MinTokensTooHigh` – The amount of tokens which could be received in exchange for the specified
        ///     `liquidity_amount` is lower than the specified `min_tokens`.
        #[pallet::weight(<T as Config>::WeightInfo::remove_liquidity())]
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

        /// Exchange currency for asset. Optionally, transfer bought asset to `recipient`. The caller can specify either:
        ///   * exact amount of currency to sell (`input_amount`) and minimum amount of tokens to buy (`min_output`), or
        ///   * exact amount of tokens to buy (`output_amount`) and maximum amount of currency to sell (`max_input`).
        ///
        /// Emit `CurrencyTradedForAsset` event on success.
        ///
        /// **Parameters:**
        ///   * `origin` – Origin for the call. Must be signed.
        ///   * `asset_id` – ID of the bought asset. An exchange for this asset must exist and have sufficient liquidity.
        ///   * `amount` – Amount of the currency and asset to trade.
        ///   * `deadline` – Number of the last block in which the transaction can be included.
        ///   * `recipient` – (Optional) account to transfer the bought tokens to.
        ///
        /// **Errors:**
        ///   * `DeadlinePassed` – Specified `deadline` is lower than the current block number.
        ///   * `ExchangeNotFound` – There is no exchange for the given `asset_id`.
        ///   * `TradeAmountIsZero` – Specified currency or token amount equals 0.
        ///   * `MinTokensTooHigh` – The amount of tokens which could be received in exchange for the specified
        ///     currency amount (`input_amount`) is lower than the specified minimum (`min_output`).
        ///   * `MaxCurrencyTooLow` – The amount of currency which must be spent to receive the specified
        ///     asset amount (`output_amount`) is higher than the specified maximum (`max_input`).
        ///   * `NotEnoughLiquidity` – There is not enough liquidity in the pool to buy the specified
        ///     amount of tokens (`output_amount`).
        ///   * `BalanceTooLow` – The available currency balance of the caller account is not enough to perform the trade.
        ///   * `Overflow` – An overflow occurred during price computation.
        #[pallet::weight(<T as Config>::WeightInfo::currency_to_asset())]
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

        /// Exchange asset for currency. Optionally, transfer bought currency to `recipient`. The caller can specify either:
        ///   * exact amount of tokes to sell (`input_amount`) and minimum amount of currency to buy (`min_output`), or
        ///   * exact amount of currency to buy (`output_amount`) and maximum amount of tokens to sell (`max_input`).
        ///
        /// Emit `AssetTradedForCurrency` event on success.
        ///
        /// **Parameters:**
        ///   * `origin` – Origin for the call. Must be signed.
        ///   * `asset_id` – ID of the sold asset. An exchange for this asset must exist and have sufficient liquidity.
        ///   * `amount` – Amount of the currency and asset to trade.
        ///   * `deadline` – Number of the last block in which the transaction can be included.
        ///   * `recipient` – (Optional) account to transfer the currency tokens to.
        ///
        /// **Errors:**
        ///   * `DeadlinePassed` – Specified `deadline` is lower than the current block number.
        ///   * `ExchangeNotFound` – There is no exchange for the given `asset_id`.
        ///   * `TradeAmountIsZero` – Specified currency or token amount equals 0.
        ///   * `MinCurrencyTooHigh` – The amount of currency which could be received in exchange for the specified
        ///     asset amount (`input_amount`) is lower than the specified minimum (`min_output`).
        ///   * `MaxTokensTooLow` – The amount of asset which must be spent to receive the specified
        ///     currency amount (`output_amount`) is higher than the specified maximum (`max_input`).
        ///   * `NotEnoughLiquidity` – There is not enough liquidity in the pool to buy the specified
        ///     amount of currency (`output_amount`).
        ///   * `NotEnoughTokens` – The available asset balance of the caller account is not enough to perform the trade.
        ///   * `Overflow` – An overflow occurred during price computation.
        #[pallet::weight(<T as Config>::WeightInfo::asset_to_currency())]
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

        /// Exchange asset for another asset. Optionally, transfer bought asset to `recipient`. The caller can specify either:
        ///   * exact amount of tokes to sell (`input_amount`) and minimum amount of tokens to buy (`min_output`), or
        ///   * exact amount of tokens to buy (`output_amount`) and maximum amount of tokens to sell (`max_input`).
        ///
        /// **Parameters:**
        ///   * `origin` – Origin for the call. Must be signed.
        ///   * `sold_asset_id` – ID of the sold asset. An exchange for this asset must exist and have sufficient liquidity.
        ///   * `bought_asset_id` – ID of the bought asset. An exchange for this asset must exist and have sufficient liquidity.
        ///   * `amount` – Amount of the assets to trade.
        ///   * `deadline` – Number of the last block in which the transaction can be included.
        ///   * `recipient` – (Optional) account to transfer the bought tokens to.
        ///
        /// **Errors:**
        ///   * `DeadlinePassed` – Specified `deadline` is lower than the current block number.
        ///   * `ExchangeNotFound` – There is no exchange for the given `sold_asset_id` or `bought_asset_id`.
        ///   * `TradeAmountIsZero` – Specified bought or sold token amount equals 0.
        ///   * `MinBoughtTokensTooHigh` – The amount of asset which could be bought in exchange for the specified
        ///     sold asset amount (`input_amount`) is lower than the specified minimum (`min_output`).
        ///   * `MaxSoldTokensTooLow` – The amount of asset which must be sold to receive the specified
        ///     bought asset amount (`output_amount`) is higher than the specified maximum (`max_input`).
        ///   * `NotEnoughLiquidity` – There is not enough liquidity in one of the pools to buy the specified amount of asset
        ///     (`output_amount`).
        ///   * `NotEnoughTokens` – The available sold asset balance of the caller account is not enough to perform the trade.
        ///   * `Overflow` – An overflow occurred during price computation.
        #[pallet::weight(<T as Config>::WeightInfo::asset_to_asset())]
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
