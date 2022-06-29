use crate as dex;
use frame_support::traits::{ConstU16, ConstU32, ConstU64, Everything, GenesisBuild};
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_runtime::testing::Header;
use sp_runtime::traits::{BlakeTwo256, IdentityLookup};

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

impl dex::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type Balance = u64;
    type MaxExchangeProviders = ConstU32<MAX_PROVIDERS>;
    type WeightInfo = ();
}

pub(crate) const ACCOUNT_A: u64 = 0;
pub(crate) const ACCOUNT_B: u64 = 1;
pub(crate) const INIT_BALANCE: u64 = 1_000_000;
pub(crate) const ASSET_ID: u32 = 100;
pub(crate) const MAX_PROVIDERS: u32 = 10;

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(ACCOUNT_A, INIT_BALANCE), (ACCOUNT_B, INIT_BALANCE)],
    }
    .assimilate_storage(&mut storage)
    .unwrap();
    pallet_assets::GenesisConfig::<Test> {
        assets: vec![(ASSET_ID, ACCOUNT_A, true, 1)],
        metadata: vec![],
        accounts: vec![
            (ASSET_ID, ACCOUNT_A, INIT_BALANCE),
            (ASSET_ID, ACCOUNT_B, INIT_BALANCE),
        ],
    }
    .assimilate_storage(&mut storage)
    .unwrap();
    let mut test_ext: sp_io::TestExternalities = storage.into();
    test_ext.execute_with(|| System::set_block_number(1));
    test_ext
}

pub(crate) fn last_event() -> dex::Event<Test> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let Event::Dex(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .last()
        .unwrap()
}
