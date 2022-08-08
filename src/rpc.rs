use crate::{AssetBalanceOf, AssetIdOf, BalanceOf, Config, ConfigHelper, Error, Pallet};
use codec::{Decode, Encode};
use scale_info::prelude::format;
use sp_std::fmt::Debug;
use sp_std::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum RpcError {
    ExchangeNotFound,
    NotEnoughLiquidity,
    Overflow,
    Unexpected(Vec<u8>),
}

pub type RpcResult<T> = Result<T, RpcError>;

impl<T: Config> From<Error<T>> for RpcError {
    fn from(err: Error<T>) -> Self {
        match err {
            Error::ExchangeNotFound => Self::ExchangeNotFound,
            Error::NotEnoughLiquidity => Self::NotEnoughLiquidity,
            Error::Overflow => Self::Overflow,
            err => Self::Unexpected(format!("{:?}", err).into_bytes()),
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Get the price for a fixed-input currency-to-asset trade,
    /// i.e. 'How much asset would I get if I paid this much currency'?
    pub fn get_currency_to_asset_input_price(
        asset_id: AssetIdOf<T>,
        currency_amount: BalanceOf<T>,
    ) -> RpcResult<AssetBalanceOf<T>> {
        let exchange = Self::get_exchange(&asset_id)?;
        let price = Self::get_input_price(
            &currency_amount,
            &exchange.currency_reserve,
            &T::asset_to_currency(exchange.token_reserve),
        )?;
        Ok(T::currency_to_asset(price))
    }

    /// Get the price for a fixed-output currency-to-asset trade,
    /// i.e. 'How much currency do I have to pay to get this much asset'?
    pub fn get_currency_to_asset_output_price(
        asset_id: AssetIdOf<T>,
        token_amount: AssetBalanceOf<T>,
    ) -> RpcResult<BalanceOf<T>> {
        let exchange = Self::get_exchange(&asset_id)?;
        let price = Self::get_output_price(
            &T::asset_to_currency(token_amount),
            &exchange.currency_reserve,
            &T::asset_to_currency(exchange.token_reserve),
        )?;
        Ok(price)
    }

    /// Get the price for a fixed-input asset-to-currency trade,
    /// i.e. 'How much currency would I get if I paid this much asset'?
    pub fn get_asset_to_currency_input_price(
        asset_id: AssetIdOf<T>,
        token_amount: AssetBalanceOf<T>,
    ) -> RpcResult<BalanceOf<T>> {
        let exchange = Self::get_exchange(&asset_id)?;
        let price = Self::get_input_price(
            &T::asset_to_currency(token_amount),
            &T::asset_to_currency(exchange.token_reserve),
            &exchange.currency_reserve,
        )?;
        Ok(price)
    }

    /// Get the price for a fixed-output currency-to-asset trade,
    /// i.e. 'How much asset do I have to pay to get this much currency'?
    pub fn get_asset_to_currency_output_price(
        asset_id: AssetIdOf<T>,
        currency_amount: BalanceOf<T>,
    ) -> RpcResult<AssetBalanceOf<T>> {
        let exchange = Self::get_exchange(&asset_id)?;
        let price = Self::get_output_price(
            &currency_amount,
            &T::asset_to_currency(exchange.token_reserve),
            &exchange.currency_reserve,
        )?;
        Ok(T::currency_to_asset(price))
    }
}
