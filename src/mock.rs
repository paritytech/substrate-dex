use crate as dex;
use frame_support::traits::{
    AsEnsureOriginWithArg, ConstU128, ConstU16, ConstU32, Everything, GenesisBuild,
};
use frame_support::{parameter_types, PalletId};
use frame_system::{EnsureRoot, EnsureSigned};
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, Identity, IdentityLookup};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Header = sp_runtime::generic::Header<u32, BlakeTwo256>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Assets: pallet_assets::{Pallet, Call, Storage, Config<T>, Event<T>},
        Dex: dex::{Pallet, Call, Storage, Event<T>},
    }
);

impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u32;
    type BlockNumber = u32;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU32<250>;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type Balance = u128;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

impl pallet_assets::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type AssetId = u32;
    type AssetIdParameter = u32;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<u64>>;
    type ForceOrigin = EnsureRoot<u64>;
    type AssetDeposit = ConstU128<1>;
    type AssetAccountDeposit = ConstU128<10>;
    type MetadataDepositBase = ConstU128<1>;
    type MetadataDepositPerByte = ConstU128<1>;
    type ApprovalDeposit = ConstU128<1>;
    type StringLimit = ConstU32<50>;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = ();
    type RemoveItemsLimit = ConstU32<5>;
    type CallbackHandle = ();
}

parameter_types! {
    pub const DexPalletId: PalletId = PalletId(*b"dex_mock");
}

impl dex::Config for Test {
    type PalletId = DexPalletId;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type AssetBalance = u128;
    type AssetToCurrencyBalance = Identity;
    type CurrencyToAssetBalance = Identity;
    type AssetId = u32;
    type Assets = Assets;
    type AssetRegistry = Assets;
    type WeightInfo = ();
    // Provider fee is 0.3%
    type ProviderFeeNumerator = ConstU128<3>;
    type ProviderFeeDenominator = ConstU128<1000>;
    type MinDeposit = ConstU128<MIN_DEPOSIT>;
}

pub(crate) const ACCOUNT_A: u64 = 0;
pub(crate) const ACCOUNT_B: u64 = 1;
pub(crate) const ACCOUNT_C: u64 = 2;
pub(crate) const INIT_BALANCE: u128 = 1_000_000_000_000_000;
pub(crate) const INIT_LIQUIDITY: u128 = 1_000_000_000_000;
pub(crate) const MIN_DEPOSIT: u128 = 1;
pub(crate) const ASSET_A: u32 = 100;
pub(crate) const ASSET_B: u32 = 101;
pub(crate) const LIQ_TOKEN_A: u32 = 200;
pub(crate) const LIQ_TOKEN_B: u32 = 201;

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (ACCOUNT_A, INIT_BALANCE),
            (ACCOUNT_B, INIT_BALANCE),
            (ACCOUNT_C, INIT_BALANCE),
        ],
    }
    .assimilate_storage(&mut storage)
    .unwrap();

    pallet_assets::GenesisConfig::<Test> {
        assets: vec![(ASSET_A, ACCOUNT_A, true, 1), (ASSET_B, ACCOUNT_B, true, 1)],
        metadata: vec![],
        accounts: vec![
            (ASSET_A, ACCOUNT_A, INIT_BALANCE),
            (ASSET_A, ACCOUNT_B, INIT_BALANCE),
            (ASSET_A, ACCOUNT_C, INIT_BALANCE),
            (ASSET_B, ACCOUNT_A, INIT_BALANCE),
            (ASSET_B, ACCOUNT_B, INIT_BALANCE),
            (ASSET_B, ACCOUNT_C, INIT_BALANCE),
        ],
    }
    .assimilate_storage(&mut storage)
    .unwrap();

    dex::GenesisConfig::<Test> {
        exchanges: vec![(ACCOUNT_A, ASSET_A, LIQ_TOKEN_A, INIT_LIQUIDITY, INIT_LIQUIDITY)],
    }
    .assimilate_storage(&mut storage)
    .unwrap();

    let mut test_ext: sp_io::TestExternalities = storage.into();
    test_ext.execute_with(|| System::set_block_number(1));
    test_ext
}

pub(crate) fn last_event() -> dex::Event<Test> {
    last_n_events(1).pop().unwrap()
}

pub(crate) fn last_n_events(n: usize) -> Vec<dex::Event<Test>> {
    let mut events: Vec<dex::Event<Test>> = System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|event| match event {
            RuntimeEvent::Dex(inner) => Some(inner),
            _ => None,
        })
        .collect();
    events.split_off(events.len() - n)
}
