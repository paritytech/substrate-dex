# DEX pallet

[![Rust check](https://github.com/Wiezzel/substrate-dex/actions/workflows/rust.yml/badge.svg)](https://github.com/Wiezzel/substrate-dex/actions/workflows/rust.yml)

## Overview

This pallet re-implements Uniswap V1 protocol for decentralized exchange of fungible assets. Please refer to the
[protocol description](https://docs.uniswap.org/protocol/V1/introduction) and
[smart contracts](https://github.com/Uniswap/v1-contracts) for more details not covered in this README.
DEX pallet allows users to create exchanges (i.e. liquidity pools), supply them with liquidity
(i.e. currency & assets), and perform trades (currency-to-asset, asset-to-currency, asset-to-asset).
More information about this is contained in the [Extrinsics](#extrinsics) section. DEX pallet also allows querying asset
prices by custom RPC methods (see [RPC](#rpc) section).    

## Main concepts

* **Assets** – Any transferable fungibles, also referred to as _tokens_.
* **Currency** – The chain's main currency/token (e.g. DOT for the relay chain, ACA for Acala).
* **Exchange** – A liquidity pool containing certain amount of an asset, and certain amount of currency. It allows users
to swap this particular asset for currency or vice versa. The asset price (i.e. exchange rate) is established dynamically
based on the currency-to-asset ratio.
* **Liquidity provider** – An account which deposits certain amount of asset and currency into an exchange.
  Providers are incentivized by receiving a fee (percentage of all transactions) paid by traders.
* **Liquidity token** – A transferable, fungible token representing an account's share in a particular liquidity pool.
It is minted when liquidity is added to the pool, and burned when liquidity is removed.

## Configuration

### Types
* `Event` – The overarching event type.
* `Currency` – The currency type.
* `AssetBalance` – The balance type for assets.
* `AssetToCurrencyBalance` – A type providing conversion from the asset balance type to the currency balance type.
* `CurrencyToAssetBalance` – A type providing conversion from the currency balance type to the asset balance type.
* `AssetId` – The asset ID type.
* `Assets` – The assets type.
* `AssetRegistry` – The liquidity tokens type.
* `WeightInfo` – Information on runtime weights.

### Constants
* `PalletId` – Pallet ID. Used for account derivation.
* `ProviderFeeNumerator` – Numerator of the fractional number representing liquidity provider fee. Should be lower than
the denominator (fees cannot exceed 100%).
* `ProviderFeeDenominator` – Denominator of the fractional number representing liquidity provider fee.
* `MinDeposit` – Minimum amount of currency which must be deposited when creating a new exchange.

## Extrinsics

### `create_exchange`
Create a new exchange. Deposit initial liquidity (currency & assets). Create a new liquidity token. Mint & transfer
to the caller account an amount of the liquidity token equal to `currency_amount`.
Emit two events on success: `ExchangeCreated` and `LiquidityAdded`.

#### Parameters:
  * `origin` – Origin for the call. Must be signed.
  * `asset_id` – ID of the asset traded on the created exchange. Asset with this ID must exist.
  * `liquidity_token_id` – ID of the liquidity token to be created. Asset with this ID must *not* exist.
  * `currency_amount` – Initial amount of the currency to deposit in the pool. Must be at least equal `MinDeposit`.
  * `token_amount` – Initial amount of tokens to deposit in the pool. Must be greater than 0.

#### Errors:
  * `AssetNotFound` – Asset with the given `asset_id` does not exist or has total supply equal 0.
  * `ExchangeAlreadyExists` – An exchange fot the specified asset already exists.
  * `TokenIdTaken` – Specified `liquidity_token_id` is already taken by another liquidity token.
  * `CurrencyAmountTooLow` – Specified `currency_amount` is lower than `MinDeposit`.
  * `TokenAmountIsZero` – Specified `token_amount` equals 0.

### `add_liquidity`
Add liquidity to an existing exchange. The caller specifies an exact amount of currency to be deposited, a maximum
amount of tokens to be deposited, and a minimum amount of liquidity tokens to receive.
Emit `LiquidityAdded` event on success.

#### Parameters:
  * `origin` – Origin for the call. Must be signed.
  * `asset_id` – ID of the deposited asset. An exchange for this asset must exist.
  * `currency_amount` – The amount of the currency to deposit in the pool. Must be greater than 0.
  * `min_liquidity` – The minimum amount of liquidity tokens to receive. Must be greater than 0.
  * `max_tokens` – The maximum amount of tokens to be deposited. Must be greater than 0.
  * `deadline` – Number of the last block in which the transaction can be included.

#### Errors:
  * `DeadlinePassed` – Specified `deadline` is lower than the current block number.
  * `ExchangeNotFound` – There is no exchange for the given `asset_id`.
  * `CurrencyAmountIsZero` – Specified `currency_amount` equals 0.
  * `MinLiquidityIsZero` – Specified `min_liquidity` equals 0.
  * `MaxTokensIsZero` – Specified `max_tokens` equals 0.
  * `BalanceTooLow` – Specified `currency_amount` is greater than the available currency balance of the caller account.
  * `NotEnoughTokens` – Specified `max_tokens` is greater than the available asset balance of the caller account.
  * `MaxTokensTooLow` – Specified `max_tokens` is too low to match the `currency_amount`. Currency and tokens need to
    be added proportionally.
  * `MinLiquidityTooHigh` – The amount of liquidity tokes which would be minted by depositing the specified
    `currency_amount` is lower than the specified `min_liquidity`.

### `remove_liquidity`
Remove liquidity from an exchange. The caller specifies the amount of liquidity tokens to burn, and minimum amounts
of currency and asset to receive. Emit `LiquidityRemoved` event on success.

#### Parameters:
  * `origin` – Origin for the call. Must be signed.
  * `asset_id` – ID of the withdrawn asset. An exchange for this asset must exist.
  * `liquidity_amount` – The amount of liquidity tokens to be burned. Must be greater than 0.
  * `min_currency` – The minimum amount of currency to receive. Must be greater than 0.
  * `min_tokens` – The minimum amount of tokens to receive. Must be greater than 0.
  * `deadline` – Number of the last block in which the transaction can be included.

#### Errors:
  * `DeadlinePassed` – Specified `deadline` is lower than the current block number.
  * `ExchangeNotFound` – There is no exchange for the given `asset_id`.
  * `LiquidityAmountIsZero` – Specified `liquidity_amount` equals 0.
  * `MinCurrencyIsZero` – Specified `min_currency` equals 0.
  * `MinTokensIsZero` – Specified `min_tokens` equals 0.
  * `ProviderLiquidityTooLow` – Specified `liquidity_amount` is greater than the liquidity token balance of the 
    caller account.
  * `MinCurrencyTooHigh` – The amount of currency which could be received in exchange for the specified
    `liquidity_amount` is lower than the specified `min_currency`.
  * `MinTokensTooHigh` – The amount of tokens which could be received in exchange for the specified
    `liquidity_amount` is lower than the specified `min_tokens`.

### `currency_to_asset`
Exchange currency for asset. Optionally, transfer bought asset to `recipient`. The caller can specify either:
  * exact amount of currency to sell (`input_amount`) and minimum amount of tokens to buy (`min_output`), or
  * exact amount of tokens to buy (`output_amount`) and maximum amount of currency to sell (`max_input`).

Emit `CurrencyTradedForAsset` event on success.

#### Parameters:
  * `origin` – Origin for the call. Must be signed.
  * `asset_id` – ID of the bought asset. An exchange for this asset must exist and have sufficient liquidity.
  * `amount` – Amount of the currency and asset to trade.
  * `deadline` – Number of the last block in which the transaction can be included.
  * `recipient` – (Optional) account to transfer the bought tokens to.

#### Errors:
  * `DeadlinePassed` – Specified `deadline` is lower than the current block number.
  * `ExchangeNotFound` – There is no exchange for the given `asset_id`.
  * `TradeAmountIsZero` – Specified currency or token amount equals 0.
  * `MinTokensTooHigh` – The amount of tokens which could be received in exchange for the specified
    currency amount (`input_amount`) is lower than the specified minimum (`min_output`).
  * `MaxCurrencyTooLow` – The amount of currency which must be spent to receive the specified
    asset amount (`output_amount`) is higher than the specified maximum (`max_input`).
  * `NotEnoughLiquidity` – There is not enough liquidity in the pool to buy the specified amount of tokens
    (`output_amount`).
  * `BalanceTooLow` – The available currency balance of the caller account is not enough to perform the trade.
  * `Overflow` – An overflow occurred during price computation.

### `asset_to_currency`
Exchange asset for currency. Optionally, transfer bought currency to `recipient`. The caller can specify either:
  * exact amount of tokes to sell (`input_amount`) and minimum amount of currency to buy (`min_output`), or
  * exact amount of currency to buy (`output_amount`) and maximum amount of tokens to sell (`max_input`).

Emit `AssetTradedForCurrency` event on success.

#### Parameters:
  * `origin` – Origin for the call. Must be signed.
  * `asset_id` – ID of the sold asset. An exchange for this asset must exist and have sufficient liquidity.
  * `amount` – Amount of the currency and asset to trade.
  * `deadline` – Number of the last block in which the transaction can be included.
  * `recipient` – (Optional) account to transfer the currency tokens to.

#### Errors:
  * `DeadlinePassed` – Specified `deadline` is lower than the current block number.
  * `ExchangeNotFound` – There is no exchange for the given `asset_id`.
  * `TradeAmountIsZero` – Specified currency or token amount equals 0.
  * `MinCurrencyTooHigh` – The amount of currency which could be received in exchange for the specified
    asset amount (`input_amount`) is lower than the specified minimum (`min_output`).
  * `MaxTokensTooLow` – The amount of asset which must be spent to receive the specified
    currency amount (`output_amount`) is higher than the specified maximum (`max_input`).
  * `NotEnoughLiquidity` – There is not enough liquidity in the pool to buy the specified amount of currency
    (`output_amount`).
  * `NotEnoughTokens` – The available asset balance of the caller account is not enough to perform the trade.
  * `Overflow` – An overflow occurred during price computation.

### `asset_to_asset`
Exchange asset for another asset. Optionally, transfer bought asset to `recipient`. The caller can specify either:
  * exact amount of tokes to sell (`input_amount`) and minimum amount of tokens to buy (`min_output`), or
  * exact amount of tokens to buy (`output_amount`) and maximum amount of tokens to sell (`max_input`).

Emit two events on success: `AssetTradedForCurrency` and `CurrencyTradedForAsset`.

#### Parameters:
  * `origin` – Origin for the call. Must be signed.
  * `sold_asset_id` – ID of the sold asset. An exchange for this asset must exist and have sufficient liquidity.
  * `bought_asset_id` – ID of the bought asset. An exchange for this asset must exist and have sufficient liquidity.
  * `amount` – Amount of the assets to trade.
  * `deadline` – Number of the last block in which the transaction can be included.
  * `recipient` – (Optional) account to transfer the bought tokens to.

#### Errors:
  * `DeadlinePassed` – Specified `deadline` is lower than the current block number.
  * `ExchangeNotFound` – There is no exchange for the given `sold_asset_id` or `bought_asset_id`.
  * `TradeAmountIsZero` – Specified bought or sold token amount equals 0.
  * `MinBoughtTokensTooHigh` – The amount of asset which could be bought in exchange for the specified
    sold asset amount (`input_amount`) is lower than the specified minimum (`min_output`).
  * `MaxSoldTokensTooLow` – The amount of asset which must be sold to receive the specified
    bought asset amount (`output_amount`) is higher than the specified maximum (`max_input`).
  * `NotEnoughLiquidity` – There is not enough liquidity in one of the pools to buy the specified amount of asset
    (`output_amount`).
  * `NotEnoughTokens` – The available sold asset balance of the caller account is not enough to perform the trade.
  * `Overflow` – An overflow occurred during price computation.

## RPC

#### Errors (for all methods):
* `ExchangeNotFound` – There is no exchange for the given `asset_id`.
* `NotEnoughLiquidity` – There is not enough liquidity in the pool to buy the specified amount of asset/currency.
  (applies only to fixed-output price queries).
* `Overflow` – An overflow occurred during price computation.
* `Unexpected` – An unexpected runtime error occurred.

### `get_currency_to_asset_input_price`
Get the price for a fixed-input currency-to-asset trade,
i.e. 'How much asset would I get if I paid this much currency'?

#### Parameters:
* `asset_id` – ID of the asset to be bought.
* `currency_amount` – The amount of currency to be spent.

### `get_currency_to_asset_output_price`
Get the price for a fixed-output currency-to-asset trade,
i.e. 'How much currency do I have to pay to get this much asset'?

#### Parameters:
* `asset_id` – ID of the asset to be bought.
* `token_amount` – The amount of tokens to be bought.

### `get_asset_to_currency_input_price`
Get the price for a fixed-input asset-to-currency trade,
i.e. 'How much currency would I get if I paid this much asset'?

#### Parameters:
* `asset_id` – ID of the asset to be sold.
* `token_amount` – The amount of tokens to be spent.

### `get_asset_to_currency_output_price`
Get the price for a fixed-output currency-to-asset trade,
i.e. 'How much asset do I have to pay to get this much currency'?

#### Parameters:
* `asset_id` – ID of the asset to be sold.
* `token_amount` – The amount of currency to be bought.
