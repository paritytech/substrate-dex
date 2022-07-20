use crate as dex;
use frame_support::traits::{ConstU16, ConstU32, ConstU64, Everything, GenesisBuild, Randomness};
use frame_support::{parameter_types, PalletId};
use frame_system::EnsureRoot;
use lazy_static::lazy_static;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use sp_core::H256;
use sp_runtime::testing::Header;
use sp_runtime::traits::{BlakeTwo256, Identity, IdentityLookup};
use std::sync::Mutex;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

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
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type Balance = u64;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ConstU64<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

impl pallet_assets::Config for Test {
    type Event = Event;
    type Balance = u64;
    type AssetId = u32;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<u64>;
    type AssetDeposit = ConstU64<1>;
    type AssetAccountDeposit = ConstU64<10>;
    type MetadataDepositBase = ConstU64<1>;
    type MetadataDepositPerByte = ConstU64<1>;
    type ApprovalDeposit = ConstU64<1>;
    type StringLimit = ConstU32<50>;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const DexPalletId: PalletId = PalletId(*b"dex_mock");
}

// Seeded random generator provides unique values but doesn't make tests flaky
pub struct TestRandomness<T>(sp_std::marker::PhantomData<T>);

lazy_static! {
    static ref RAND: Mutex<StdRng> = Mutex::new(StdRng::seed_from_u64(07_07_2022));
}

impl<T: frame_system::Config> Randomness<H256, T::BlockNumber> for TestRandomness<T> {
    fn random(_subject: &[u8]) -> (H256, T::BlockNumber) {
        let mut random_hash = H256::default();
        RAND.lock().unwrap().fill_bytes(&mut random_hash.0);
        (random_hash, frame_system::Pallet::<T>::block_number())
    }
}

impl dex::Config for Test {
    type PalletId = DexPalletId;
    type Event = Event;
    type Currency = Balances;
    type AssetBalance = u64;
    type AssetToCurrencyBalance = Identity;
    type CurrencyToAssetBalance = Identity;
    type AssetId = u32;
    type Assets = Assets;
    type AssetRegistry = Assets;
    type Randomness = TestRandomness<Test>;
    type WeightInfo = ();
    // Provider fee is 0.3%
    type ProviderFeeNumerator = ConstU64<3>;
    type ProviderFeeDenominator = ConstU64<1000>;
}

pub(crate) const ACCOUNT_A: u64 = 0;
pub(crate) const ACCOUNT_B: u64 = 1;
pub(crate) const ACCOUNT_C: u64 = 2;
pub(crate) const INIT_BALANCE: u64 = 1_000_000_000_000_000;
pub(crate) const ASSET_A: u32 = 100;
pub(crate) const ASSET_B: u32 = 101;

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
    let mut test_ext: sp_io::TestExternalities = storage.into();
    test_ext.execute_with(|| System::set_block_number(1));
    test_ext.execute_with(|| Dex::create_exchange(Origin::signed(ACCOUNT_A), ASSET_A).unwrap());
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
            Event::Dex(inner) => Some(inner),
            _ => None,
        })
        .collect();
    events.split_off(events.len() - n)
}
