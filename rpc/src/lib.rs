use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error as RpcError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::generic::BlockId;
use sp_runtime::traits::MaybeDisplay;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;

pub use pallet_dex_rpc_runtime_api::{DexApi as DexRuntimeApi, RpcError as DexRpcError};

const RUNTIME_ERROR: i32 = 1;
const EXCHANGE_NOT_FOUND: i32 = 2;
const NOT_ENOUGH_LIQUIDITY: i32 = 3;
const OVERFLOW: i32 = 4;

#[rpc(client, server)]
pub trait DexApi<BlockHash, AssetId, Balance, AssetBalance> {
    #[method(name = "dex_get_currency_to_asset_input_price")]
    fn get_currency_to_asset_input_price(
        &self,
        asset_id: AssetId,
        currency_amount: Balance,
        at: Option<BlockHash>,
    ) -> RpcResult<AssetBalance>;

    #[method(name = "dex_get_currency_to_asset_output_price")]
    fn get_currency_to_asset_output_price(
        &self,
        asset_id: AssetId,
        token_amount: AssetBalance,
        at: Option<BlockHash>,
    ) -> RpcResult<Balance>;

    #[method(name = "dex_get_asset_to_currency_input_price")]
    fn get_asset_to_currency_input_price(
        &self,
        asset_id: AssetId,
        token_amount: AssetBalance,
        at: Option<BlockHash>,
    ) -> RpcResult<Balance>;

    #[method(name = "dex_get_asset_to_currency_output_price")]
    fn get_asset_to_currency_output_price(
        &self,
        asset_id: AssetId,
        currency_amount: Balance,
        at: Option<BlockHash>,
    ) -> RpcResult<AssetBalance>;
}

pub struct Dex<Client, Block> {
    client: Arc<Client>,
    _marker: PhantomData<Block>,
}

type HashOf<Block> = <Block as sp_runtime::traits::Block>::Hash;

impl<Client, Block> Dex<Client, Block>
where
    Block: sp_runtime::traits::Block,
    Client: HeaderBackend<Block>,
{
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }

    #[inline(always)]
    fn block_id(&self, block_hash: Option<HashOf<Block>>) -> BlockId<Block> {
        // If the block hash is not supplied assume the best block.
        let block_hash = block_hash.unwrap_or_else(|| self.client.info().best_hash);
        BlockId::hash(block_hash)
    }
}

#[async_trait]
impl<Client, Block, AssetId, Balance, AssetBalance>
    DexApiServer<HashOf<Block>, AssetId, Balance, AssetBalance> for Dex<Client, Block>
where
    Block: sp_runtime::traits::Block,
    Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    Client::Api: DexRuntimeApi<Block, AssetId, Balance, AssetBalance>,
    AssetId: Codec + MaybeDisplay + Copy + Send + Sync + 'static,
    Balance: Codec + MaybeDisplay + Copy + Send + Sync + 'static,
    AssetBalance: Codec + MaybeDisplay + Copy + Send + Sync + 'static,
{
    fn get_currency_to_asset_input_price(
        &self,
        asset_id: AssetId,
        currency_amount: Balance,
        at: Option<Block::Hash>,
    ) -> RpcResult<AssetBalance> {
        let at = self.block_id(at);
        self.client
            .runtime_api()
            .get_currency_to_asset_input_price(&at, asset_id, currency_amount)
            .map_err(runtime_error)?
            .map_err(dex_rpc_error)
    }

    fn get_currency_to_asset_output_price(
        &self,
        asset_id: AssetId,
        token_amount: AssetBalance,
        at: Option<Block::Hash>,
    ) -> RpcResult<Balance> {
        let at = self.block_id(at);
        self.client
            .runtime_api()
            .get_currency_to_asset_output_price(&at, asset_id, token_amount)
            .map_err(runtime_error)?
            .map_err(dex_rpc_error)
    }

    fn get_asset_to_currency_input_price(
        &self,
        asset_id: AssetId,
        token_amount: AssetBalance,
        at: Option<Block::Hash>,
    ) -> RpcResult<Balance> {
        let at = self.block_id(at);
        self.client
            .runtime_api()
            .get_asset_to_currency_input_price(&at, asset_id, token_amount)
            .map_err(runtime_error)?
            .map_err(dex_rpc_error)
    }

    fn get_asset_to_currency_output_price(
        &self,
        asset_id: AssetId,
        currency_amount: Balance,
        at: Option<Block::Hash>,
    ) -> RpcResult<AssetBalance> {
        let at = self.block_id(at);
        self.client
            .runtime_api()
            .get_asset_to_currency_output_price(&at, asset_id, currency_amount)
            .map_err(runtime_error)?
            .map_err(dex_rpc_error)
    }
}

fn runtime_error(err: impl Debug) -> RpcError {
    CallError::Custom(ErrorObject::owned(
        RUNTIME_ERROR,
        "Runtime error",
        Some(format!("{:?}", err)),
    ))
    .into()
}

fn dex_rpc_error(err: DexRpcError) -> RpcError {
    let (code, message, data) = match err {
        DexRpcError::ExchangeNotFound => (EXCHANGE_NOT_FOUND, "Exchange not found", None),
        DexRpcError::NotEnoughLiquidity => (NOT_ENOUGH_LIQUIDITY, "Not enough liquidity", None),
        DexRpcError::Overflow => (OVERFLOW, "Overflow", None),
        DexRpcError::Unexpected(msg) => (RUNTIME_ERROR, "Runtime error", Some(msg)),
    };
    CallError::Custom(ErrorObject::owned(code, message, data)).into()
}
