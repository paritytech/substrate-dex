use crate::mock::*;
use crate::{Error, Exchange};
use frame_support::{assert_noop, assert_ok, BoundedBTreeMap};

#[test]
fn create_exchange() {
    new_test_ext().execute_with(|| {
        assert_ok!(Dex::create_exchange(Origin::signed(ACCOUNT_A), ASSET_B));
        assert_eq!(
            Dex::exchanges(ASSET_B).unwrap(),
            Exchange {
                asset_id: ASSET_B,
                total_liquidity: 0u64,
                currency_reserve: 0u64,
                token_reserve: 0u64,
                balances: BoundedBTreeMap::new()
            }
        );
        assert_eq!(last_event(), crate::Event::ExchangeCreated(ASSET_B));
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
            1_000u64,
            0u64, // `min_liquidity` is ignored if there's no liquidity yet
            1_000u64,
        ));
        let exchange = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.total_liquidity, 1_000u64);
        assert_eq!(exchange.currency_reserve, 1_000u64);
        assert_eq!(exchange.token_reserve, 1_000u64);
        let balance = exchange.balances.get(&ACCOUNT_A).unwrap();
        assert_eq!(balance, &1_000u64);
        assert_eq!(
            last_event(),
            crate::Event::LiquidityAdded(ACCOUNT_A, ASSET_A, 1_000u64, 1_000u64, 1_000u64)
        );

        assert_ok!(Dex::add_liquidity(
            Origin::signed(ACCOUNT_B),
            ASSET_A,
            500u64,
            500u64,
            1_000u64,
        ));
        let exchange = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.total_liquidity, 1_500u64);
        assert_eq!(exchange.currency_reserve, 1_500u64);
        assert_eq!(exchange.token_reserve, 1_501u64);
        let balance = exchange.balances.get(&ACCOUNT_B).unwrap();
        assert_eq!(balance, &500u64);
        assert_eq!(
            last_event(),
            crate::Event::LiquidityAdded(ACCOUNT_B, ASSET_A, 500u64, 501u64, 500u64)
        );
    })
}

#[test]
fn add_liquidity_unsigned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(Origin::none(), ASSET_A, 1_000u64, 1_000u64, 1_000u64,),
            frame_support::error::BadOrigin
        );
    })
}

#[test]
fn add_liquidity_zero_currency() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 0u64, 1_000u64, 1_000u64,),
            Error::<Test>::CurrencyAmountIsZero
        );
    })
}

#[test]
fn add_liquidity_zero_tokens() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(Origin::signed(ACCOUNT_A), ASSET_A, 1_000u64, 1_000u64, 0u64,),
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
                INIT_BALANCE + 1u64,
                1_000u64,
                1_000u64,
            ),
            Error::<Test>::BalanceTooLow
        );
    })
}

#[test]
fn add_liquidity_asset_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(
                Origin::signed(ACCOUNT_A),
                2137,
                1_000u64,
                1_000u64,
                1_000u64,
            ),
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
                1_000u64,
                1_000u64,
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
            Dex::add_liquidity(
                Origin::signed(ACCOUNT_A),
                ASSET_B,
                1_000u64,
                1_000u64,
                1_000u64,
            ),
            Error::<Test>::ExchangeNotFound
        );
    })
}

#[test]
fn add_liquidity_max_providers_reached() {
    new_test_ext().execute_with(|| {
        // Max providers is 2, so accounts A&B will fill in all slots.
        Dex::add_liquidity(
            Origin::signed(ACCOUNT_A),
            ASSET_A,
            1_000u64,
            1_000u64,
            1_000u64,
        )
        .unwrap();
        Dex::add_liquidity(
            Origin::signed(ACCOUNT_B),
            ASSET_A,
            1_000u64,
            1_000u64,
            1_001u64,
        )
        .unwrap();
        assert_noop!(
            Dex::add_liquidity(
                Origin::signed(ACCOUNT_C),
                ASSET_A,
                1_000u64,
                1_000u64,
                1_001u64,
            ),
            Error::<Test>::MaxProvidersReached
        );
    })
}

#[test]
fn add_liquidity_zero_min_liquidity() {
    new_test_ext().execute_with(|| {
        // `min_liquidity` is ignored if existing liquidity is 0, so we need to add some first.
        Dex::add_liquidity(
            Origin::signed(ACCOUNT_A),
            ASSET_A,
            1_000u64,
            1_000u64,
            1_000u64,
        )
        .unwrap();
        assert_noop!(
            Dex::add_liquidity(Origin::signed(ACCOUNT_B), ASSET_A, 1_000u64, 0u64, 1_001u64,),
            Error::<Test>::MinLiquidityIsZero
        );
    })
}

#[test]
fn add_liquidity_max_tokens_too_low() {
    new_test_ext().execute_with(|| {
        // `max_tokens` is always enough if existing liquidity is 0, so we need to add some first.
        Dex::add_liquidity(
            Origin::signed(ACCOUNT_A),
            ASSET_A,
            1_000u64,
            1_000u64,
            1_000u64,
        )
        .unwrap();
        assert_noop!(
            Dex::add_liquidity(
                Origin::signed(ACCOUNT_B),
                ASSET_A,
                1_000u64,
                1_000u64,
                10u64,
            ),
            Error::<Test>::MaxTokensTooLow
        );
    })
}

#[test]
fn add_liquidity_min_liquidity_too_high() {
    new_test_ext().execute_with(|| {
        // `min_liquidity` is ignored if existing liquidity is 0, so we need to add some first.
        Dex::add_liquidity(
            Origin::signed(ACCOUNT_A),
            ASSET_A,
            1_000u64,
            1_000u64,
            1_000u64,
        )
        .unwrap();
        assert_noop!(
            Dex::add_liquidity(
                Origin::signed(ACCOUNT_B),
                ASSET_A,
                1_000u64,
                10_000u64,
                1_001u64,
            ),
            Error::<Test>::MinLiquidityTooHigh
        );
    })
}
