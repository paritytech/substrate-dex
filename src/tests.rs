use crate::mock::*;
use crate::pallet::Config as DexConfig;
use crate::{Error, Exchange};
use frame_support::sp_runtime::traits::AccountIdConversion;
use frame_support::{assert_noop, assert_ok, BoundedBTreeMap};

#[test]
fn create_exchange() {
    new_test_ext().execute_with(|| {
        assert_ok!(Dex::create_exchange(Origin::signed(ACCOUNT_A), ASSET_B));
        let exchange = Dex::exchanges(ASSET_B).unwrap();
        assert_eq!(exchange.asset_id, ASSET_B);
        assert_eq!(exchange.currency_reserve, 0);
        assert_eq!(exchange.token_reserve, 0);
        assert_eq!(Assets::total_supply(exchange.liquidity_token_id), 0);
        let event = last_event();
        assert!(
            matches!(last_event(), crate::Event::ExchangeCreated(asset, _) if asset == ASSET_B)
        );
    })
}

#[test]
fn create_exchange_unsigned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::create_exchange(Origin::none(), 2137),
            frame_support::error::BadOrigin
        );
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

#[test]
fn create_exchange_already_exists() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::create_exchange(Origin::signed(ACCOUNT_A), ASSET_A),
            Error::<Test>::ExchangeAlreadyExists
        );
    })
}

#[test]
fn add_liquidity() {
    new_test_ext().execute_with(|| {
        assert_ok!(Dex::add_liquidity(
            Origin::signed(ACCOUNT_A),
            ASSET_A,
            1_000,
            0, // `min_liquidity` is ignored if there's no liquidity yet
            1_000,
        ));

        let exchange = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, 1_000);
        assert_eq!(exchange.token_reserve, 1_000);
        assert_eq!(Balances::free_balance(ACCOUNT_A), INIT_BALANCE - 1_000);
        assert_eq!(
            Assets::maybe_balance(ASSET_A, &ACCOUNT_A),
            Some(INIT_BALANCE - 1_000)
        );
        assert_eq!(
            Assets::maybe_balance(exchange.liquidity_token_id, &ACCOUNT_A),
            Some(1_000)
        );
        assert_eq!(Assets::total_supply(exchange.liquidity_token_id), 1_000);
        let pallet_account = <Test as DexConfig>::PalletId::get().into_account_truncating();
        assert_eq!(Balances::free_balance(pallet_account), 1_000);
        assert_eq!(Assets::maybe_balance(ASSET_A, &pallet_account), Some(1_000));
        assert_eq!(
            last_event(),
            crate::Event::LiquidityAdded(ACCOUNT_A, ASSET_A, 1_000, 1_000, 1_000)
        );

        assert_ok!(Dex::add_liquidity(
            Origin::signed(ACCOUNT_B),
            ASSET_A,
            500,
            500,
            1_000,
        ));

        let exchange = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, 1_500);
        assert_eq!(exchange.token_reserve, 1_501);
        assert_eq!(Balances::free_balance(ACCOUNT_B), INIT_BALANCE - 500);
        assert_eq!(
            Assets::maybe_balance(ASSET_A, &ACCOUNT_B),
            Some(INIT_BALANCE - 501)
        );
        assert_eq!(
            Assets::maybe_balance(exchange.liquidity_token_id, &ACCOUNT_B),
            Some(500)
        );
        assert_eq!(Assets::total_supply(exchange.liquidity_token_id), 1_500);
        assert_eq!(Balances::free_balance(pallet_account), 1_500);
        assert_eq!(Assets::maybe_balance(ASSET_A, &pallet_account), Some(1_501));
        assert_eq!(
            last_event(),
            crate::Event::LiquidityAdded(ACCOUNT_B, ASSET_A, 500, 501, 500)
        );
    })
}

#[test]
fn add_liquidity_unsigned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(Origin::none(), ASSET_A, 1_000, 1_000, 1_000,),
            frame_support::error::BadOrigin
        );
    })
}

#[test]
fn add_liquidity_zero_currency() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 0, 1_000, 1_000,),
            Error::<Test>::CurrencyAmountIsZero
        );
    })
}

#[test]
fn add_liquidity_zero_tokens() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 0,),
            Error::<Test>::MaxTokensIsZero
        );
    })
}

#[test]
fn add_liquidity_balance_too_low() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(
                Origin::signed(ACCOUNT_A),
                ASSET_A,
                INIT_BALANCE + 1,
                1_000,
                1_000,
            ),
            Error::<Test>::BalanceTooLow
        );
    })
}

#[test]
fn add_liquidity_asset_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(Origin::signed(ACCOUNT_A), 2137, 1_000, 1_000, 1_000,),
            Error::<Test>::AssetNotFound
        );
    })
}

#[test]
fn add_liquidity_not_enough_tokens() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(
                Origin::signed(ACCOUNT_A),
                ASSET_A,
                1_000,
                1_000,
                INIT_BALANCE + 1,
            ),
            Error::<Test>::NotEnoughTokens
        );
    })
}

#[test]
fn add_liquidity_exchange_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_B, 1_000, 1_000, 1_000,),
            Error::<Test>::ExchangeNotFound
        );
    })
}

#[test]
fn add_liquidity_zero_min_liquidity() {
    new_test_ext().execute_with(|| {
        // `min_liquidity` is ignored if existing liquidity is 0, so we need to add some first.
        Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000).unwrap();
        assert_noop!(
            Dex::add_liquidity(Origin::signed(ACCOUNT_B), ASSET_A, 1_000, 0, 1_001,),
            Error::<Test>::MinLiquidityIsZero
        );
    })
}

#[test]
fn add_liquidity_max_tokens_too_low() {
    new_test_ext().execute_with(|| {
        // `max_tokens` is always enough if existing liquidity is 0, so we need to add some first.
        Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000).unwrap();
        assert_noop!(
            Dex::add_liquidity(Origin::signed(ACCOUNT_B), ASSET_A, 1_000, 1_000, 10,),
            Error::<Test>::MaxTokensTooLow
        );
    })
}

#[test]
fn add_liquidity_min_liquidity_too_high() {
    new_test_ext().execute_with(|| {
        // `min_liquidity` is ignored if existing liquidity is 0, so we need to add some first.
        Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000).unwrap();
        assert_noop!(
            Dex::add_liquidity(Origin::signed(ACCOUNT_B), ASSET_A, 1_000, 10_000, 1_001,),
            Error::<Test>::MinLiquidityTooHigh
        );
    })
}

#[test]
fn remove_liquidity() {
    new_test_ext().execute_with(|| {
        Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000).unwrap();
        assert_ok!(Dex::remove_liquidity(
            Origin::signed(ACCOUNT_A),
            ASSET_A,
            500,
            500,
            500
        ));
        let exchange = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, 500);
        assert_eq!(exchange.token_reserve, 500);
        assert_eq!(Assets::total_supply(exchange.liquidity_token_id), 500);
        let event = last_event();
        assert_eq!(
            event,
            crate::Event::LiquidityRemoved(ACCOUNT_A, ASSET_A, 500, 500, 500)
        );
    });
}

#[test]
fn remove_liquidity_unsigned() {
    new_test_ext().execute_with(|| {
        Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000).unwrap();
        assert_noop!(
            Dex::remove_liquidity(Origin::none(), ASSET_A, 500, 500, 500),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn remove_zero_liquidity() {
    new_test_ext().execute_with(|| {
        Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000).unwrap();
        assert_noop!(
            Dex::remove_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 0, 500, 500),
            crate::Error::<Test>::LiquidityAmountIsZero
        );
    });
}

#[test]
fn remove_liquidity_min_currency_zero() {
    new_test_ext().execute_with(|| {
        Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000).unwrap();
        assert_noop!(
            Dex::remove_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 500, 0, 500),
            crate::Error::<Test>::MinCurrencyIsZero
        );
    });
}

#[test]
fn remove_liquidity_min_tokens_zero() {
    new_test_ext().execute_with(|| {
        Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000).unwrap();
        assert_noop!(
            Dex::remove_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 500, 500, 0),
            crate::Error::<Test>::MinTokensIsZero
        );
    });
}

#[test]
fn remove_liquidity_exchange_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::remove_liquidity(Origin::signed(ACCOUNT_A), ASSET_B, 500, 500, 500),
            crate::Error::<Test>::ExchangeNotFound
        );
    });
}

#[test]
fn remove_liquidity_provider_liquidity_too_low() {
    new_test_ext().execute_with(|| {
        Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000).unwrap();
        assert_noop!(
            Dex::remove_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_500, 500, 500),
            crate::Error::<Test>::ProviderLiquidityTooLow
        );
    });
}

#[test]
fn remove_liquidity_min_currency_too_high() {
    new_test_ext().execute_with(|| {
        Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000).unwrap();
        assert_noop!(
            Dex::remove_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 500, 1_500, 500),
            crate::Error::<Test>::MinCurrencyTooHigh
        );
    });
}

#[test]
fn remove_liquidity_min_tokens_too_high() {
    new_test_ext().execute_with(|| {
        Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000).unwrap();
        assert_noop!(
            Dex::remove_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 500, 500, 1_500),
            crate::Error::<Test>::MinTokensTooHigh
        );
    });
}
