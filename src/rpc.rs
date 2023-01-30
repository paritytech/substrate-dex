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
            err => Self::Unexpected(format!("{err:?}").into_bytes()),
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Get the output amount for a fixed-input currency-to-asset trade,
    /// i.e. 'How much asset would I get if I paid this much currency'?
    pub fn get_currency_to_asset_output_amount(
        asset_id: AssetIdOf<T>,
        currency_amount: BalanceOf<T>,
    ) -> RpcResult<AssetBalanceOf<T>> {
        let exchange = Self::get_exchange(&asset_id)?;
        let output_amount = Self::get_output_amount(
            &currency_amount,
            &exchange.currency_reserve,
            &T::asset_to_currency(exchange.token_reserve),
        )?;
        Ok(T::currency_to_asset(output_amount))
    }

    /// Get the input amount for a fixed-output currency-to-asset trade,
    /// i.e. 'How much currency do I have to pay to get this much asset'?
    pub fn get_currency_to_asset_input_amount(
        asset_id: AssetIdOf<T>,
        token_amount: AssetBalanceOf<T>,
    ) -> RpcResult<BalanceOf<T>> {
        let exchange = Self::get_exchange(&asset_id)?;
        let input_amount = Self::get_input_amount(
            &T::asset_to_currency(token_amount),
            &exchange.currency_reserve,
            &T::asset_to_currency(exchange.token_reserve),
        )?;
        Ok(input_amount)
    }

    /// Get the output amount for a fixed-input asset-to-currency trade,
    /// i.e. 'How much currency would I get if I paid this much asset'?
    pub fn get_asset_to_currency_output_amount(
        asset_id: AssetIdOf<T>,
        token_amount: AssetBalanceOf<T>,
    ) -> RpcResult<BalanceOf<T>> {
        let exchange = Self::get_exchange(&asset_id)?;
        let output_amount = Self::get_output_amount(
            &T::asset_to_currency(token_amount),
            &T::asset_to_currency(exchange.token_reserve),
            &exchange.currency_reserve,
        )?;
        Ok(output_amount)
    }

    /// Get the input amount for a fixed-output currency-to-asset trade,
    /// i.e. 'How much asset do I have to pay to get this much currency'?
    pub fn get_asset_to_currency_input_amount(
        asset_id: AssetIdOf<T>,
        currency_amount: BalanceOf<T>,
    ) -> RpcResult<AssetBalanceOf<T>> {
        let exchange = Self::get_exchange(&asset_id)?;
        let input_amount = Self::get_input_amount(
            &currency_amount,
            &T::asset_to_currency(exchange.token_reserve),
            &exchange.currency_reserve,
        )?;
        Ok(T::currency_to_asset(input_amount))
    }
}

#[cfg(test)]
mod tests {
    use crate::mock::*;
    use crate::rpc::RpcError;
    use crate::{AssetBalanceOf, AssetIdOf, BalanceOf, Exchange, Exchanges};
    use frame_support::assert_noop;

    #[test]
    fn get_currency_to_asset_output_amount_exchange_not_found() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                Dex::get_currency_to_asset_output_amount(u32::MAX, 0),
                RpcError::ExchangeNotFound
            );
        })
    }

    #[test]
    fn get_currency_to_asset_output_amount_overflow() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                Dex::get_currency_to_asset_output_amount(ASSET_A, u128::MAX),
                RpcError::Overflow
            );
        })
    }

    #[test]
    fn get_currency_to_asset_output_amount() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                996_999,
                Dex::get_currency_to_asset_output_amount(ASSET_A, 1_000_000).unwrap(),
            );
        })
    }

    #[test]
    fn get_currency_to_asset_input_amount_exchange_not_found() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                Dex::get_currency_to_asset_input_amount(u32::MAX, 0),
                RpcError::ExchangeNotFound
            );
        })
    }

    #[test]
    fn get_currency_to_asset_input_amount_not_enough_liquidity() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                Dex::get_currency_to_asset_input_amount(ASSET_A, u128::MAX),
                RpcError::NotEnoughLiquidity
            );
        })
    }

    #[test]
    fn get_currency_to_asset_input_amount_overflow() {
        new_test_ext().execute_with(|| {
            // Update exchange reserves to cause overflow
            max_exchange_reserves(ASSET_A);
            assert_noop!(Dex::get_currency_to_asset_input_amount(ASSET_A, 1), RpcError::Overflow);
        })
    }

    #[test]
    fn get_currency_to_asset_input_amount() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                1_003_011,
                Dex::get_currency_to_asset_input_amount(ASSET_A, 1_000_000).unwrap(),
            );
        })
    }

    #[test]
    fn get_asset_to_currency_output_amount_exchange_not_found() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                Dex::get_asset_to_currency_output_amount(u32::MAX, 0),
                RpcError::ExchangeNotFound
            );
        })
    }

    #[test]
    fn get_asset_to_currency_output_amount_overflow() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                Dex::get_asset_to_currency_output_amount(ASSET_A, u128::MAX),
                RpcError::Overflow
            );
        })
    }

    #[test]
    fn get_asset_to_currency_output_amount() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                996_999,
                Dex::get_asset_to_currency_output_amount(ASSET_A, 1_000_000).unwrap(),
            );
        })
    }

    #[test]
    fn get_asset_to_currency_input_amount_exchange_not_found() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                Dex::get_asset_to_currency_input_amount(u32::MAX, 0),
                RpcError::ExchangeNotFound
            );
        })
    }

    #[test]
    fn get_asset_to_currency_input_amount_not_enough_liquidity() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                Dex::get_asset_to_currency_input_amount(ASSET_A, u128::MAX),
                RpcError::NotEnoughLiquidity
            );
        })
    }

    #[test]
    fn get_asset_to_currency_input_amount_overflow() {
        new_test_ext().execute_with(|| {
            // Update exchange reserves to cause overflow
            max_exchange_reserves(ASSET_A);
            assert_noop!(
                Dex::get_asset_to_currency_input_amount(ASSET_A, INIT_LIQUIDITY - 1),
                RpcError::Overflow
            );
        })
    }

    #[test]
    fn get_asset_to_currency_input_amount() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                1_003_011,
                Dex::get_asset_to_currency_input_amount(ASSET_A, 1_000_000).unwrap(),
            );
        })
    }

    fn max_exchange_reserves(asset_id: AssetIdOf<Test>) {
        Exchanges::<Test>::insert(
            asset_id,
            Exchange::<AssetIdOf<Test>, BalanceOf<Test>, AssetBalanceOf<Test>> {
                asset_id,
                currency_reserve: u128::MAX,
                token_reserve: u128::MAX,
                liquidity_token_id: LIQ_TOKEN_A,
            },
        );
    }
}
