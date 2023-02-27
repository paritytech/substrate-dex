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
const DATA: [u8; 15] = [
    117, 110, 101, 120, 112, 101, 99, 116, 101, 100, 32, 100, 97, 116, 97,
];

fn assert(error: Error, code: i32, message: &str, data: Option<&[u8]>) {
    assert!(matches!(error, Error::Call(e) if matches!(&e, CallError::Custom(e)
    if e.code() == code && e.message() == message &&
        e.data().map(|v| v.get().to_string()) == data.map(|d| format!("{d:?}").replace(' ', "")))));
}

#[tokio::test]
async fn get_currency_to_asset_output_amount_with_exchange_not_found() {
    let expectation = Expectation::GetCurrencyToAssetOutputAmount(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::ExchangeNotFound),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_output_amount(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert(error, EXCHANGE_NOT_FOUND, EXCHANGE_NOT_FOUND_MESSAGE, None)
}

#[tokio::test]
async fn get_currency_to_asset_output_amount_with_not_enough_liquidity() {
    let expectation = Expectation::GetCurrencyToAssetOutputAmount(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::NotEnoughLiquidity),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_output_amount(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert(error, NOT_ENOUGH_LIQUIDITY, NOT_ENOUGH_LIQUIDITY_MESSAGE, None)
}

#[tokio::test]
async fn get_currency_to_asset_output_amount_with_overflow() {
    let expectation = Expectation::GetCurrencyToAssetOutputAmount(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::Overflow),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_output_amount(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert(error, OVERFLOW, OVERFLOW_MESSAGE, None)
}

#[tokio::test]
async fn get_currency_to_asset_output_amount_with_unexpected() {
    let expectation = Expectation::GetCurrencyToAssetOutputAmount(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::Unexpected(DATA.into())),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_output_amount(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert(error, RUNTIME_ERROR, RUNTIME_ERROR_MESSAGE, Some(&DATA))
}

#[tokio::test]
async fn get_currency_to_asset_output_amount_with_success() {
    let expectation = Expectation::GetCurrencyToAssetOutputAmount(ASSET, CURRENCY_AMOUNT, Ok(100));

    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let result = api
        .get_currency_to_asset_output_amount(ASSET, CURRENCY_AMOUNT, None)
        .unwrap();

    assert_eq!(100, result);
}

#[tokio::test]
async fn get_currency_to_asset_input_amount_with_exchange_not_found() {
    let expectation = Expectation::GetCurrencyToAssetInputAmount(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::ExchangeNotFound),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_input_amount(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert(error, EXCHANGE_NOT_FOUND, EXCHANGE_NOT_FOUND_MESSAGE, None)
}

#[tokio::test]
async fn get_currency_to_asset_input_amount_with_not_enough_liquidity() {
    let expectation = Expectation::GetCurrencyToAssetInputAmount(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::NotEnoughLiquidity),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_input_amount(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert(error, NOT_ENOUGH_LIQUIDITY, NOT_ENOUGH_LIQUIDITY_MESSAGE, None)
}

#[tokio::test]
async fn get_currency_to_asset_input_amount_with_overflow() {
    let expectation =
        Expectation::GetCurrencyToAssetInputAmount(ASSET, TOKEN_AMOUNT, Err(RpcError::Overflow));
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_input_amount(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert(error, OVERFLOW, OVERFLOW_MESSAGE, None)
}

#[tokio::test]
async fn get_currency_to_asset_input_amount_with_unexpected() {
    let expectation = Expectation::GetCurrencyToAssetInputAmount(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::Unexpected(DATA.into())),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_currency_to_asset_input_amount(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert(error, RUNTIME_ERROR, RUNTIME_ERROR_MESSAGE, Some(&DATA))
}

#[tokio::test]
async fn get_currency_to_asset_input_amount_with_success() {
    let expectation = Expectation::GetCurrencyToAssetInputAmount(ASSET, TOKEN_AMOUNT, Ok(100));

    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let result = api
        .get_currency_to_asset_input_amount(ASSET, TOKEN_AMOUNT, None)
        .unwrap();

    assert_eq!(100, result);
}

#[tokio::test]
async fn get_asset_to_currency_output_amount_with_exchange_not_found() {
    let expectation = Expectation::GetAssetToCurrencyOutputAmount(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::ExchangeNotFound),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_output_amount(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert(error, EXCHANGE_NOT_FOUND, EXCHANGE_NOT_FOUND_MESSAGE, None)
}

#[tokio::test]
async fn get_asset_to_currency_output_amount_with_not_enough_liquidity() {
    let expectation = Expectation::GetAssetToCurrencyOutputAmount(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::NotEnoughLiquidity),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_output_amount(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert(error, NOT_ENOUGH_LIQUIDITY, NOT_ENOUGH_LIQUIDITY_MESSAGE, None)
}

#[tokio::test]
async fn get_asset_to_currency_output_amount_with_overflow() {
    let expectation =
        Expectation::GetAssetToCurrencyOutputAmount(ASSET, TOKEN_AMOUNT, Err(RpcError::Overflow));
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_output_amount(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert(error, OVERFLOW, OVERFLOW_MESSAGE, None)
}

#[tokio::test]
async fn get_asset_to_currency_output_amount_with_unexpected() {
    let expectation = Expectation::GetAssetToCurrencyOutputAmount(
        ASSET,
        TOKEN_AMOUNT,
        Err(RpcError::Unexpected(DATA.into())),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_output_amount(ASSET, TOKEN_AMOUNT, None)
        .unwrap_err();

    assert(error, RUNTIME_ERROR, RUNTIME_ERROR_MESSAGE, Some(&DATA))
}

#[tokio::test]
async fn get_asset_to_currency_output_amount_with_success() {
    let expectation = Expectation::GetAssetToCurrencyOutputAmount(ASSET, TOKEN_AMOUNT, Ok(100));

    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let result = api
        .get_asset_to_currency_output_amount(ASSET, TOKEN_AMOUNT, None)
        .unwrap();

    assert_eq!(100, result);
}

#[tokio::test]
async fn get_asset_to_currency_input_amount_with_exchange_not_found() {
    let expectation = Expectation::GetAssetToCurrencyInputAmount(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::ExchangeNotFound),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_input_amount(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert(error, EXCHANGE_NOT_FOUND, EXCHANGE_NOT_FOUND_MESSAGE, None)
}

#[tokio::test]
async fn get_asset_to_currency_input_amount_with_not_enough_liquidity() {
    let expectation = Expectation::GetAssetToCurrencyInputAmount(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::NotEnoughLiquidity),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_input_amount(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert(error, NOT_ENOUGH_LIQUIDITY, NOT_ENOUGH_LIQUIDITY_MESSAGE, None)
}

#[tokio::test]
async fn get_asset_to_currency_input_amount_with_overflow() {
    let expectation =
        Expectation::GetAssetToCurrencyInputAmount(ASSET, CURRENCY_AMOUNT, Err(RpcError::Overflow));
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_input_amount(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert(error, OVERFLOW, OVERFLOW_MESSAGE, None)
}

#[tokio::test]
async fn get_asset_to_currency_input_amount_with_unexpected() {
    let expectation = Expectation::GetAssetToCurrencyInputAmount(
        ASSET,
        CURRENCY_AMOUNT,
        Err(RpcError::Unexpected(DATA.into())),
    );
    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let error = api
        .get_asset_to_currency_input_amount(ASSET, CURRENCY_AMOUNT, None)
        .unwrap_err();

    assert(error, RUNTIME_ERROR, RUNTIME_ERROR_MESSAGE, Some(&DATA))
}

#[tokio::test]
async fn get_asset_to_currency_input_amount_with_success() {
    let expectation = Expectation::GetAssetToCurrencyInputAmount(ASSET, CURRENCY_AMOUNT, Ok(100));

    let client = Arc::new(TestApi::new(expectation));
    let api = Dex::new(client);

    let result = api
        .get_asset_to_currency_input_amount(ASSET, CURRENCY_AMOUNT, None)
        .unwrap();

    assert_eq!(100, result);
}

mod mock {
    use crate::tests::{AssetBalance, AssetId, Balance, RpcResult};
    use pallet_dex_rpc_runtime_api::DexApi as DexRuntimeApi;
    use sp_api::{ApiRef, ProvideRuntimeApi};
    use sp_blockchain::HeaderBackend;
    use sp_runtime::traits::{Block as BlockT, NumberFor, Zero};
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
            _id: <Block as BlockT>::Hash,
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
            _id: <Block as BlockT>::Hash,
        ) -> Result<sc_client_api::blockchain::BlockStatus, sp_blockchain::Error> {
            Ok(sc_client_api::blockchain::BlockStatus::Unknown)
        }

        fn number(
            &self,
            _hash: Block::Hash,
        ) -> Result<Option<NumberFor<Block>>, sp_blockchain::Error> {
            Ok(None)
        }

        fn hash(
            &self,
            _number: NumberFor<Block>,
        ) -> Result<Option<Block::Hash>, sp_blockchain::Error> {
            Ok(None)
        }
    }

    pub struct TestRuntimeApi {
        pub(super) call: Expectation,
    }

    sp_api::mock_impl_runtime_apis! {
        // A simple mock implementation to compare provided values with expected
        impl DexRuntimeApi<Block, AssetId, Balance, AssetBalance> for TestRuntimeApi {
            fn get_currency_to_asset_output_amount(asset_id: AssetId, currency_amount: Balance) -> RpcResult<AssetBalance> {
                match &self.call {
                    Expectation::GetCurrencyToAssetOutputAmount ( expected_asset, expected_amount, result)
                        if asset_id == *expected_asset && currency_amount == *expected_amount => result.clone(),
                    _ => panic!()
                }
            }

            fn get_currency_to_asset_input_amount(asset_id: AssetId, token_amount: AssetBalance) -> RpcResult<Balance> {
                match &self.call {
                    Expectation::GetCurrencyToAssetInputAmount ( expected_asset, expected_amount, result)
                        if asset_id == *expected_asset && token_amount == *expected_amount => result.clone(),
                    _ => panic!()
                }
            }

            fn get_asset_to_currency_output_amount(asset_id: AssetId, token_amount: AssetBalance) -> RpcResult<Balance>{
                match &self.call {
                    Expectation::GetAssetToCurrencyOutputAmount ( expected_asset, expected_amount, result)
                        if asset_id == *expected_asset && token_amount == *expected_amount => result.clone(),
                    _ => panic!()
                }
            }

            fn get_asset_to_currency_input_amount(asset_id: AssetId, currency_amount: Balance) -> RpcResult<AssetBalance>{
                match &self.call {
                    Expectation::GetAssetToCurrencyInputAmount ( expected_asset, expected_amount, result)
                        if asset_id == *expected_asset && currency_amount == *expected_amount => result.clone(),
                    _ => panic!()
                }
            }
        }
    }

    #[allow(clippy::enum_variant_names)]
    #[derive(PartialEq, Debug, Clone)]
    pub(crate) enum Expectation {
        GetCurrencyToAssetOutputAmount(AssetId, Balance, RpcResult<AssetBalance>),
        GetCurrencyToAssetInputAmount(AssetId, AssetBalance, RpcResult<Balance>),
        GetAssetToCurrencyOutputAmount(AssetId, AssetBalance, RpcResult<Balance>),
        GetAssetToCurrencyInputAmount(AssetId, Balance, RpcResult<AssetBalance>),
    }
}
