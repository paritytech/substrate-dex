use crate::mock::*;
use crate::{Error, Exchange};
use frame_support::{assert_noop, assert_ok, BoundedBTreeMap};

#[test]
fn genesis_config() {
    new_test_ext().execute_with(|| {
        assert_eq!(<crate::pallet::Exchanges<Test>>::iter().count(), 0);
    });
}

#[test]
fn create_exchange() {
    new_test_ext().execute_with(|| {
        assert_ok!(Dex::create_exchange(Origin::signed(ACCOUNT_A), ASSET_ID));
        assert_eq!(
            Dex::exchanges(ASSET_ID).unwrap(),
            Exchange {
                asset_id: ASSET_ID,
                total_liquidity: 0u64,
                currency_reserve: 0u64,
                token_reserve: 0u64,
                balances: BoundedBTreeMap::new()
            }
        );
        assert_eq!(last_event(), crate::Event::ExchangeCreated(ASSET_ID));
    })
}

#[test]
fn create_exchange_asset_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::create_exchange(Origin::signed(ACCOUNT_A), 2137),
            Error::<Test>::AssetNotFound
        );
    })
}
