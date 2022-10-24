use super::*;
use jsonrpsee::core::Error;
use mock::*;
use pallet_dex::rpc::RpcError;
use std::sync::Arc;

type AssetId = u32;
type Balance = u128;
type AssetBalance = u64;
type RpcResult<T> = Result<T, RpcError>;

const ASSET: AssetId = 1;
const CURRENCY_AMOUNT: Balance = 100;
const TOKEN_AMOUNT: AssetBalance = 100;
const EXCHANGE_NOT_FOUND_MESSAGE: &str = "Exchange not found";
const NOT_ENOUGH_LIQUIDITY_MESSAGE: &str = "Not enough liquidity";
const OVERFLOW_MESSAGE: &str = "Overflow";
const RUNTIME_ERROR_MESSAGE: &str = "Runtime error";

fn assert_exchange_not_found(error: Error) {
    assert!(matches!(error, Error::Call(e) if matches!(&e, CallError::Custom(e)
        if e.code() == EXCHANGE_NOT_FOUND && e.message() == EXCHANGE_NOT_FOUND_MESSAGE && e.data().is_none())));
}

fn assert_not_enough_liquidity(error: Error) {
    assert!(matches!(error, Error::Call(e) if matches!(&e, CallError::Custom(e)
        if e.code() == NOT_ENOUGH_LIQUIDITY && e.message() == NOT_ENOUGH_LIQUIDITY_MESSAGE && e.data().is_none())));
}

fn assert_overflow(error: Error) {
    assert!(matches!(error, Error::Call(e) if matches!(&e, CallError::Custom(e)
        if e.code() == OVERFLOW && e.message() == OVERFLOW_MESSAGE && e.data().is_none())));
}

fn assert_unexpected(error: Error) {
    assert!(matches!(error, Error::Call(e) if matches!(&e, CallError::Custom(e)
        if e.code() == RUNTIME_ERROR && e.message() == RUNTIME_ERROR_MESSAGE && e.data().is_some())));
}

#[tokio::test]
async fn get_currency_to_asset_input_price_with_exchange_not_found() {
    let expectation = Expectation::GetCurrencyToAssetInputPrice(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::ExchangeNotFound),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_input_price(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert_exchange_not_found(error)
}

#[tokio::test]
async fn get_currency_to_asset_input_price_with_not_enough_liquidity() {
    let expectation = Expectation::GetCurrencyToAssetInputPrice(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::NotEnoughLiquidity),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_input_price(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert_not_enough_liquidity(error)
}

#[tokio::test]
async fn get_currency_to_asset_input_price_with_overflow() {
    let expectation =
        Expectation::GetCurrencyToAssetInputPrice(ASSET, CURRENCY_AMOUNT, Err(RpcError::Overflow));
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_input_price(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert_overflow(error)
}

#[tokio::test]
async fn get_currency_to_asset_input_price_with_unexpected() {
    let expectation = Expectation::GetCurrencyToAssetInputPrice(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::Unexpected("unexpected asset".as_bytes().into())),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_input_price(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert_unexpected(error)
}

#[tokio::test]
async fn get_currency_to_asset_input_price_with_success() {
    let expectation = Expectation::GetCurrencyToAssetInputPrice(ASSET, CURRENCY_AMOUNT, Ok(100));

    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let result = api
        .get_currency_to_asset_input_price(ASSET, CURRENCY_AMOUNT, None)
        .unwrap();

    assert_eq!(100, result);
}

#[tokio::test]
async fn get_currency_to_asset_output_price_with_exchange_not_found() {
    let expectation = Expectation::GetCurrencyToAssetOutputPrice(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::ExchangeNotFound),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_output_price(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert_exchange_not_found(error)
}

#[tokio::test]
async fn get_currency_to_asset_output_price_with_not_enough_liquidity() {
    let expectation = Expectation::GetCurrencyToAssetOutputPrice(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::NotEnoughLiquidity),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_output_price(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert_not_enough_liquidity(error)
}

#[tokio::test]
async fn get_currency_to_asset_output_price_with_overflow() {
    let expectation =
        Expectation::GetCurrencyToAssetOutputPrice(ASSET, TOKEN_AMOUNT, Err(RpcError::Overflow));
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_output_price(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert_overflow(error)
}

#[tokio::test]
async fn get_currency_to_asset_output_price_with_unexpected() {
    let expectation = Expectation::GetCurrencyToAssetOutputPrice(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::Unexpected("unexpected asset".as_bytes().into())),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_output_price(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert_unexpected(error)
}

#[tokio::test]
async fn get_currency_to_asset_output_price_with_success() {
    let expectation = Expectation::GetCurrencyToAssetOutputPrice(ASSET, TOKEN_AMOUNT, Ok(100));

    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let result = api
        .get_currency_to_asset_output_price(ASSET, TOKEN_AMOUNT, None)
        .unwrap();

    assert_eq!(100, result);
}

#[tokio::test]
async fn get_asset_to_currency_input_price_with_exchange_not_found() {
    let expectation = Expectation::GetAssetToCurrencyInputPrice(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::ExchangeNotFound),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_input_price(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert_exchange_not_found(error)
}

#[tokio::test]
async fn get_asset_to_currency_input_price_with_not_enough_liquidity() {
    let expectation = Expectation::GetAssetToCurrencyInputPrice(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::NotEnoughLiquidity),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_input_price(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert_not_enough_liquidity(error)
}

#[tokio::test]
async fn get_asset_to_currency_input_price_with_overflow() {
    let expectation =
        Expectation::GetAssetToCurrencyInputPrice(ASSET, TOKEN_AMOUNT, Err(RpcError::Overflow));
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_input_price(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert_overflow(error)
}

#[tokio::test]
async fn get_asset_to_currency_input_price_with_unexpected() {
    let expectation = Expectation::GetAssetToCurrencyInputPrice(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::Unexpected("unexpected asset".as_bytes().into())),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_input_price(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert_unexpected(error)
}

#[tokio::test]
async fn get_asset_to_currency_input_price_with_success() {
    let expectation = Expectation::GetAssetToCurrencyInputPrice(ASSET, TOKEN_AMOUNT, Ok(100));

    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let result = api
        .get_asset_to_currency_input_price(ASSET, TOKEN_AMOUNT, None)
        .unwrap();

    assert_eq!(100, result);
}

#[tokio::test]
async fn get_asset_to_currency_output_price_with_exchange_not_found() {
    let expectation = Expectation::GetAssetToCurrencyOutputPrice(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::ExchangeNotFound),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_output_price(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert_exchange_not_found(error)
}

#[tokio::test]
async fn get_asset_to_currency_output_price_with_not_enough_liquidity() {
    let expectation = Expectation::GetAssetToCurrencyOutputPrice(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::NotEnoughLiquidity),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_output_price(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert_not_enough_liquidity(error)
}

#[tokio::test]
async fn get_asset_to_currency_output_price_with_overflow() {
    let expectation =
        Expectation::GetAssetToCurrencyOutputPrice(ASSET, CURRENCY_AMOUNT, Err(RpcError::Overflow));
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_output_price(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert_overflow(error)
}

#[tokio::test]
async fn get_asset_to_currency_output_price_with_unexpected() {
    let expectation = Expectation::GetAssetToCurrencyOutputPrice(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::Unexpected("unexpected asset".as_bytes().into())),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_output_price(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert_unexpected(error)
}

#[tokio::test]
async fn get_asset_to_currency_output_price_with_success() {
    let expectation = Expectation::GetAssetToCurrencyOutputPrice(ASSET, CURRENCY_AMOUNT, Ok(100));

    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let result = api
        .get_asset_to_currency_output_price(ASSET, CURRENCY_AMOUNT, None)
        .unwrap();

    assert_eq!(100, result);
}

mod mock {
    use crate::tests::{AssetBalance, AssetId, Balance, RpcResult};
    use pallet_dex_rpc_runtime_api::DexApi as DexRuntimeApi;
    use sp_api::{ApiRef, ProvideRuntimeApi};
    use sp_blockchain::HeaderBackend;
    use sp_runtime::{
        generic::BlockId,
        traits::{Block as BlockT, NumberFor, Zero},
    };
    use substrate_test_runtime_client::runtime::Block;

    pub struct TestApi {
        pub(super) expectation: Expectation,
    }

    impl TestApi {
        pub(super) fn new(expectation: Expectation) -> Self {
            Self { expectation }
        }
    }

    impl ProvideRuntimeApi<Block> for TestApi {
        type Api = TestRuntimeApi;

        fn runtime_api(&self) -> ApiRef<Self::Api> {
            TestRuntimeApi {
                call: self.expectation.clone(),
            }
            .into()
        }
    }

    impl<Block: BlockT> HeaderBackend<Block> for TestApi {
        fn header(
            &self,
            _id: BlockId<Block>,
        ) -> Result<Option<Block::Header>, sp_blockchain::Error> {
            Ok(None)
        }

        fn info(&self) -> sc_client_api::blockchain::Info<Block> {
            sc_client_api::blockchain::Info {
                best_hash: Default::default(),
                best_number: Zero::zero(),
                finalized_hash: Default::default(),
                finalized_number: Zero::zero(),
                genesis_hash: Default::default(),
                number_leaves: Default::default(),
                finalized_state: None,
                block_gap: None,
            }
        }

        fn status(
            &self,
            _id: BlockId<Block>,
        ) -> std::result::Result<sc_client_api::blockchain::BlockStatus, sp_blockchain::Error>
        {
            Ok(sc_client_api::blockchain::BlockStatus::Unknown)
        }

        fn number(
            &self,
            _hash: Block::Hash,
        ) -> std::result::Result<Option<NumberFor<Block>>, sp_blockchain::Error> {
            Ok(None)
        }

        fn hash(
            &self,
            _number: NumberFor<Block>,
        ) -> std::result::Result<Option<Block::Hash>, sp_blockchain::Error> {
            Ok(None)
        }
    }

    pub struct TestRuntimeApi {
        pub(super) call: Expectation,
    }

    sp_api::mock_impl_runtime_apis! {
        // A simple mock implementation to compare provided values with expected
        impl DexRuntimeApi<Block, AssetId, Balance, AssetBalance> for TestRuntimeApi {
            fn get_currency_to_asset_input_price(asset_id: AssetId, currency_amount: Balance) -> RpcResult<AssetBalance> {
                match &self.call {
                    Expectation::GetCurrencyToAssetInputPrice ( expected_asset, expected_amount, result)
                        if asset_id == *expected_asset && currency_amount == *expected_amount => result.clone(),
                    _ => panic!()
                }
            }

            fn get_currency_to_asset_output_price(asset_id: AssetId, token_amount: AssetBalance) -> RpcResult<Balance> {
                match &self.call {
                    Expectation::GetCurrencyToAssetOutputPrice ( expected_asset, expected_amount, result)
                        if asset_id == *expected_asset && token_amount == *expected_amount => result.clone(),
                    _ => panic!()
                }
            }

            fn get_asset_to_currency_input_price(asset_id: AssetId, token_amount: AssetBalance) -> RpcResult<Balance>{
                match &self.call {
                    Expectation::GetAssetToCurrencyInputPrice ( expected_asset, expected_amount, result)
                        if asset_id == *expected_asset && token_amount == *expected_amount => result.clone(),
                    _ => panic!()
                }
            }

            fn get_asset_to_currency_output_price(asset_id: AssetId, currency_amount: Balance) -> RpcResult<AssetBalance>{
                match &self.call {
                    Expectation::GetAssetToCurrencyOutputPrice ( expected_asset, expected_amount, result)
                        if asset_id == *expected_asset && currency_amount == *expected_amount => result.clone(),
                    _ => panic!()
                }
            }
        }
    }

    #[allow(clippy::enum_variant_names)]
    #[derive(PartialEq, Debug, Clone)]
    pub(crate) enum Expectation {
        GetCurrencyToAssetInputPrice(AssetId, Balance, RpcResult<AssetBalance>),
        GetCurrencyToAssetOutputPrice(AssetId, AssetBalance, RpcResult<Balance>),
        GetAssetToCurrencyInputPrice(AssetId, AssetBalance, RpcResult<Balance>),
        GetAssetToCurrencyOutputPrice(AssetId, Balance, RpcResult<AssetBalance>),
    }
}
