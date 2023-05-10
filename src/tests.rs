use crate::mock::*;
use crate::pallet::ConfigHelper;
use crate::{Error, TradeAmount};
use frame_support::{
    assert_noop, assert_ok,
    traits::{fungibles::Mutate, Currency},
};

#[test]
fn create_exchange() {
    new_test_ext().execute_with(|| {
        assert_ok!(Dex::create_exchange(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_B,
            LIQ_TOKEN_B,
            1,
            1
        ));
        let exchange = Dex::exchanges(ASSET_B).unwrap();
        assert_eq!(exchange.asset_id, ASSET_B);
        assert_eq!(exchange.currency_reserve, 1);
        assert_eq!(exchange.token_reserve, 1);
        assert_eq!(Assets::total_supply(exchange.liquidity_token_id), 1);
        assert!(
            matches!(last_event(), crate::Event::ExchangeCreated(asset, _) if asset == ASSET_B)
        );
    })
}

#[test]
fn create_exchange_unsigned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::create_exchange(RuntimeOrigin::none(), ASSET_A, LIQ_TOKEN_A, 1, 1),
            frame_support::error::BadOrigin
        );
    })
}

#[test]
fn create_exchange_currency_amount_too_low() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::create_exchange(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, LIQ_TOKEN_A, 0, 1),
            Error::<Test>::CurrencyAmountTooLow
        );
    })
}

#[test]
fn create_exchange_token_amount_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::create_exchange(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, LIQ_TOKEN_A, 1, 0),
            Error::<Test>::TokenAmountIsZero
        );
    })
}

#[test]
fn create_exchange_asset_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::create_exchange(RuntimeOrigin::signed(ACCOUNT_A), 2137, LIQ_TOKEN_A, 1, 1),
            Error::<Test>::AssetNotFound
        );
    })
}

#[test]
fn create_exchange_already_exists() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::create_exchange(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, LIQ_TOKEN_A, 1, 1),
            Error::<Test>::ExchangeAlreadyExists
        );
    })
}

#[test]
fn create_exchange_token_id_taken() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::create_exchange(RuntimeOrigin::signed(ACCOUNT_A), ASSET_B, LIQ_TOKEN_A, 1, 1),
            Error::<Test>::TokenIdTaken
        );
    })
}

#[test]
fn add_liquidity() {
    new_test_ext().execute_with(|| {
        assert_ok!(Dex::add_liquidity(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_A,
            1_000,
            1_000,
            1_001,
            1,
        ));

        let exchange = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, INIT_LIQUIDITY + 1_000);
        assert_eq!(exchange.token_reserve, INIT_LIQUIDITY + 1_001);
        assert_eq!(Balances::free_balance(ACCOUNT_B), INIT_BALANCE - 1_000);
        assert_eq!(Assets::maybe_balance(ASSET_A, ACCOUNT_B), Some(INIT_BALANCE - 1_001));
        assert_eq!(Assets::maybe_balance(exchange.liquidity_token_id, ACCOUNT_B), Some(1_000));
        assert_eq!(Assets::total_supply(exchange.liquidity_token_id), INIT_LIQUIDITY + 1_000);
        let pallet_account = Test::pallet_account();
        assert_eq!(Balances::free_balance(pallet_account), INIT_LIQUIDITY + 1_000);
        assert_eq!(Assets::maybe_balance(ASSET_A, pallet_account), Some(INIT_LIQUIDITY + 1_001));
        assert_eq!(
            last_event(),
            crate::Event::LiquidityAdded(ACCOUNT_B, ASSET_A, 1_000, 1_001, 1_000)
        );
    })
}

#[test]
fn add_liquidity_unsigned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(RuntimeOrigin::none(), ASSET_A, 1_000, 1_000, 1_000, 1),
            frame_support::error::BadOrigin
        );
    })
}

#[test]
fn add_liquidity_deadline_passed() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 1_000, 0),
            Error::<Test>::DeadlinePassed
        );
    })
}

#[test]
fn add_liquidity_zero_currency() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 0, 1_000, 1_000, 1),
            Error::<Test>::CurrencyAmountIsZero
        );
    })
}

#[test]
fn add_liquidity_zero_tokens() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 1_000, 1_000, 0, 1),
            Error::<Test>::MaxTokensIsZero
        );
    })
}

#[test]
fn add_liquidity_balance_too_low() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(
                RuntimeOrigin::signed(ACCOUNT_A),
                ASSET_A,
                INIT_BALANCE + 1,
                1_000,
                1_000,
                1,
            ),
            Error::<Test>::BalanceTooLow
        );
    })
}

#[test]
fn add_liquidity_asset_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(RuntimeOrigin::signed(ACCOUNT_A), 2137, 1_000, 1_000, 1_000, 1),
            Error::<Test>::AssetNotFound
        );
    })
}

#[test]
fn add_liquidity_not_enough_tokens() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(
                RuntimeOrigin::signed(ACCOUNT_A),
                ASSET_A,
                1_000,
                1_000,
                INIT_BALANCE + 1,
                1,
            ),
            Error::<Test>::NotEnoughTokens
        );
    })
}

#[test]
fn add_liquidity_exchange_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_B, 1_000, 1_000, 1_000, 1),
            Error::<Test>::ExchangeNotFound
        );
    })
}

#[test]
fn add_liquidity_zero_min_liquidity() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(RuntimeOrigin::signed(ACCOUNT_B), ASSET_A, 1_000, 0, 1_001, 1),
            Error::<Test>::MinLiquidityIsZero
        );
    })
}

#[test]
fn add_liquidity_max_tokens_too_low() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(RuntimeOrigin::signed(ACCOUNT_B), ASSET_A, 1_000, 1_000, 10, 1),
            Error::<Test>::MaxTokensTooLow
        );
    })
}

#[test]
fn add_liquidity_min_liquidity_too_high() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::add_liquidity(RuntimeOrigin::signed(ACCOUNT_B), ASSET_A, 1_000, 10_000, 1_001, 1),
            Error::<Test>::MinLiquidityTooHigh
        );
    })
}

#[test]
fn remove_liquidity() {
    new_test_ext().execute_with(|| {
        assert_ok!(Dex::remove_liquidity(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_A,
            500,
            500,
            500,
            1,
        ));
        let exchange = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, INIT_LIQUIDITY - 500);
        assert_eq!(exchange.token_reserve, INIT_LIQUIDITY - 500);
        assert_eq!(Balances::free_balance(ACCOUNT_A), INIT_BALANCE - INIT_LIQUIDITY + 500);
        assert_eq!(
            Assets::maybe_balance(ASSET_A, ACCOUNT_A),
            Some(INIT_BALANCE - INIT_LIQUIDITY + 500)
        );
        assert_eq!(
            Assets::maybe_balance(exchange.liquidity_token_id, ACCOUNT_A),
            Some(INIT_LIQUIDITY - 500)
        );
        assert_eq!(Assets::total_supply(exchange.liquidity_token_id), INIT_LIQUIDITY - 500);
        let pallet_account = Test::pallet_account();
        assert_eq!(Balances::free_balance(pallet_account), INIT_LIQUIDITY - 500);
        assert_eq!(Assets::maybe_balance(ASSET_A, pallet_account), Some(INIT_LIQUIDITY - 500));
        assert_eq!(last_event(), crate::Event::LiquidityRemoved(ACCOUNT_A, ASSET_A, 500, 500, 500));
    });
}

#[test]
fn remove_liquidity_unsigned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::remove_liquidity(RuntimeOrigin::none(), ASSET_A, 500, 500, 500, 1),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn remove_liquidity_deadline_passed() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 500, 500, 500, 0),
            Error::<Test>::DeadlinePassed
        );
    });
}

#[test]
fn remove_zero_liquidity() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 0, 500, 500, 1),
            crate::Error::<Test>::LiquidityAmountIsZero
        );
    });
}

#[test]
fn remove_liquidity_min_currency_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 500, 0, 500, 1),
            crate::Error::<Test>::MinCurrencyIsZero
        );
    });
}

#[test]
fn remove_liquidity_min_tokens_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 500, 500, 0, 1),
            crate::Error::<Test>::MinTokensIsZero
        );
    });
}

#[test]
fn remove_liquidity_exchange_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_B, 500, 500, 500, 1),
            crate::Error::<Test>::ExchangeNotFound
        );
    });
}

#[test]
fn remove_liquidity_provider_liquidity_too_low() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::remove_liquidity(
                RuntimeOrigin::signed(ACCOUNT_A),
                ASSET_A,
                INIT_LIQUIDITY + 500,
                INIT_LIQUIDITY + 500,
                INIT_LIQUIDITY + 500,
                1
            ),
            crate::Error::<Test>::ProviderLiquidityTooLow
        );
    });
}

#[test]
fn remove_liquidity_min_currency_too_high() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 500, 1_500, 500, 1),
            crate::Error::<Test>::MinCurrencyTooHigh
        );
    });
}

#[test]
fn remove_liquidity_min_tokens_too_high() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 500, 500, 1_500, 1),
            crate::Error::<Test>::MinTokensTooHigh
        );
    });
}

#[test]
fn currency_to_asset_fixed_input() {
    new_test_ext().execute_with(|| {
        let curr_amount = 500;
        let token_amount = 498; // currency amount (500) - provider fee (0.3%) should be ~498

        assert_ok!(Dex::currency_to_asset(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_A,
            TradeAmount::FixedInput {
                input_amount: curr_amount,
                min_output: token_amount
            },
            1,
            None
        ));

        let exchange = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, INIT_LIQUIDITY + curr_amount);
        assert_eq!(exchange.token_reserve, INIT_LIQUIDITY - token_amount);
        assert_eq!(Balances::free_balance(ACCOUNT_B), INIT_BALANCE - curr_amount);
        assert_eq!(Assets::maybe_balance(ASSET_A, ACCOUNT_B), Some(INIT_BALANCE + token_amount));
        let pallet_account = Test::pallet_account();
        assert_eq!(Balances::free_balance(pallet_account), INIT_LIQUIDITY + curr_amount);
        assert_eq!(
            Assets::maybe_balance(ASSET_A, pallet_account),
            Some(INIT_LIQUIDITY - token_amount)
        );
        assert_eq!(
            last_event(),
            crate::Event::CurrencyTradedForAsset(
                ASSET_A,
                ACCOUNT_B,
                ACCOUNT_B,
                curr_amount,
                token_amount,
            )
        );
    });
}

#[test]
fn currency_to_asset_fixed_output() {
    new_test_ext().execute_with(|| {
        let curr_amount = 500;
        let token_amount = 498; // currency amount (500) - provider fee (0.3%) should be ~498

        assert_ok!(Dex::currency_to_asset(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_A,
            TradeAmount::FixedOutput {
                max_input: curr_amount,
                output_amount: token_amount,
            },
            1,
            None
        ));

        let exchange = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, INIT_LIQUIDITY + curr_amount);
        assert_eq!(exchange.token_reserve, INIT_LIQUIDITY - token_amount);
        assert_eq!(Balances::free_balance(ACCOUNT_B), INIT_BALANCE - curr_amount);
        assert_eq!(Assets::maybe_balance(ASSET_A, ACCOUNT_B), Some(INIT_BALANCE + token_amount));
        let pallet_account = Test::pallet_account();
        assert_eq!(Balances::free_balance(pallet_account), INIT_LIQUIDITY + curr_amount);
        assert_eq!(
            Assets::maybe_balance(ASSET_A, pallet_account),
            Some(INIT_LIQUIDITY - token_amount)
        );
        assert_eq!(
            last_event(),
            crate::Event::CurrencyTradedForAsset(
                ASSET_A,
                ACCOUNT_B,
                ACCOUNT_B,
                curr_amount,
                token_amount,
            )
        );
    });
}

#[test]
fn currency_to_asset_unsigned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::currency_to_asset(
                RuntimeOrigin::none(),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: 1,
                    min_output: 1
                },
                1,
                None
            ),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn currency_to_asset_deadline_passed() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::currency_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: 1,
                    min_output: 1
                },
                0,
                None
            ),
            crate::Error::<Test>::DeadlinePassed
        );
    });
}

#[test]
fn currency_to_asset_currency_amount_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::currency_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: 0,
                    min_output: 100
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn currency_to_asset_min_tokens_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::currency_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: 100,
                    min_output: 0
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn currency_to_asset_max_currency_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::currency_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedOutput {
                    max_input: 0,
                    output_amount: 100
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn currency_to_asset_token_amount_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::currency_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedOutput {
                    max_input: 100,
                    output_amount: 0
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn currency_to_asset_balance_too_low() {
    new_test_ext().execute_with(|| {
        let currency_amount = 500;
        let min_tokens = 498; // currency amount (500) - provider fee (0.3%) should be ~498

        <Test as crate::Config>::Currency::make_free_balance_be(&ACCOUNT_B, 1);
        assert_noop!(
            Dex::currency_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: currency_amount,
                    min_output: min_tokens,
                },
                1,
                None
            ),
            crate::Error::<Test>::BalanceTooLow
        );
    });
}

#[test]
fn currency_to_asset_exchange_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::currency_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_B,
                TradeAmount::FixedInput {
                    input_amount: 1,
                    min_output: 1
                },
                1,
                None
            ),
            crate::Error::<Test>::ExchangeNotFound
        );
    });
}

#[test]
fn currency_to_asset_min_tokens_too_high() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::currency_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: 10,
                    min_output: 50
                },
                1,
                None
            ),
            crate::Error::<Test>::MinTokensTooHigh
        );
    });
}

#[test]
fn currency_to_asset_max_currency_too_low() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::currency_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedOutput {
                    max_input: 10,
                    output_amount: 50
                },
                1,
                None
            ),
            crate::Error::<Test>::MaxCurrencyTooLow
        );
    });
}

#[test]
fn currency_to_asset_not_enough_liquidity() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::currency_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedOutput {
                    max_input: INIT_LIQUIDITY + 1000,
                    output_amount: INIT_LIQUIDITY + 1000
                },
                1,
                None
            ),
            crate::Error::<Test>::NotEnoughLiquidity
        );
    });
}

#[test]
fn currency_to_asset_transfer() {
    new_test_ext().execute_with(|| {
        let curr_amount = 500;
        let token_amount = 498; // currency amount (500) - provider fee (0.3%) should be ~498

        assert_ok!(Dex::currency_to_asset(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_A,
            TradeAmount::FixedInput {
                input_amount: curr_amount,
                min_output: token_amount,
            },
            1,
            Some(ACCOUNT_C)
        ));

        assert_eq!(Balances::free_balance(ACCOUNT_B), INIT_BALANCE - curr_amount);
        assert_eq!(Assets::maybe_balance(ASSET_A, ACCOUNT_B), Some(INIT_BALANCE));
        assert_eq!(Assets::maybe_balance(ASSET_A, ACCOUNT_C), Some(INIT_BALANCE + token_amount));
        assert_eq!(
            last_event(),
            crate::Event::CurrencyTradedForAsset(
                ASSET_A,
                ACCOUNT_B,
                ACCOUNT_C,
                curr_amount,
                token_amount,
            )
        );
    });
}

#[test]
fn asset_to_currency_fixed_input() {
    new_test_ext().execute_with(|| {
        let token_amount = 500;
        let curr_amount = 498; // token amount (500) - provider fee (0.3%) should be ~498

        assert_ok!(Dex::asset_to_currency(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_A,
            TradeAmount::FixedInput {
                input_amount: token_amount,
                min_output: curr_amount
            },
            1,
            None
        ));

        let exchange = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, INIT_LIQUIDITY - curr_amount);
        assert_eq!(exchange.token_reserve, INIT_LIQUIDITY + token_amount);
        assert_eq!(Balances::free_balance(ACCOUNT_B), INIT_BALANCE + curr_amount);
        assert_eq!(Assets::maybe_balance(ASSET_A, ACCOUNT_B), Some(INIT_BALANCE - token_amount));
        let pallet_account = Test::pallet_account();
        assert_eq!(Balances::free_balance(pallet_account), INIT_LIQUIDITY - curr_amount);
        assert_eq!(
            Assets::maybe_balance(ASSET_A, pallet_account),
            Some(INIT_LIQUIDITY + token_amount)
        );
        assert_eq!(
            last_event(),
            crate::Event::AssetTradedForCurrency(
                ASSET_A,
                ACCOUNT_B,
                ACCOUNT_B,
                curr_amount,
                token_amount,
            )
        );
    });
}

#[test]
fn asset_to_currency_unsigned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_currency(
                RuntimeOrigin::none(),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: 1,
                    min_output: 1
                },
                1,
                None
            ),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn asset_to_currency_deadline_passed() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_currency(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: 1,
                    min_output: 1
                },
                0,
                None
            ),
            crate::Error::<Test>::DeadlinePassed
        );
    });
}

#[test]
fn asset_to_currency_min_currency_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_currency(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: 100,
                    min_output: 0
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn asset_to_currency_token_amount_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_currency(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: 0,
                    min_output: 100
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn asset_to_currency_currency_amount_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_currency(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedOutput {
                    max_input: 100,
                    output_amount: 0
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn asset_to_currency_max_tokens_is_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_currency(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedOutput {
                    max_input: 0,
                    output_amount: 100
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn asset_to_currency_not_enough_tokens() {
    new_test_ext().execute_with(|| {
        let token_amount = 500;
        let min_currency = 498; // token amount (500) - provider fee (0.3%) should be ~498

        <Test as crate::Config>::Assets::burn_from(ASSET_A, &ACCOUNT_B, INIT_BALANCE).unwrap();
        assert_noop!(
            Dex::asset_to_currency(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: token_amount,
                    min_output: min_currency
                },
                1,
                None
            ),
            crate::Error::<Test>::NotEnoughTokens
        );
    });
}

#[test]
fn asset_to_currency_exchange_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_currency(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_B,
                TradeAmount::FixedInput {
                    input_amount: 1,
                    min_output: 1
                },
                1,
                None
            ),
            crate::Error::<Test>::ExchangeNotFound
        );
    });
}

#[test]
fn asset_to_currency_min_currency_too_high() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_currency(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: 10,
                    min_output: 50
                },
                1,
                None
            ),
            crate::Error::<Test>::MinCurrencyTooHigh
        );
    });
}

#[test]
fn asset_to_currency_max_tokens_too_low() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_currency(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedOutput {
                    output_amount: 50,
                    max_input: 10
                },
                1,
                None
            ),
            crate::Error::<Test>::MaxTokensTooLow
        );
    });
}

#[test]
fn asset_to_currency_not_enough_liquidity() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_currency(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                TradeAmount::FixedOutput {
                    output_amount: INIT_LIQUIDITY + 1000,
                    max_input: INIT_LIQUIDITY + 1000
                },
                1,
                None
            ),
            crate::Error::<Test>::NotEnoughLiquidity
        );
    });
}

#[test]
fn asset_to_currency_transfer() {
    new_test_ext().execute_with(|| {
        let token_amount = 500;
        let curr_amount = 498; // token amount (500) - provider fee (0.3%) should be ~498

        assert_ok!(Dex::asset_to_currency(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_A,
            TradeAmount::FixedInput {
                input_amount: token_amount,
                min_output: curr_amount
            },
            1,
            Some(ACCOUNT_C)
        ));

        assert_eq!(Assets::maybe_balance(ASSET_A, ACCOUNT_B), Some(INIT_BALANCE - token_amount));
        assert_eq!(Balances::free_balance(ACCOUNT_B), INIT_BALANCE);
        assert_eq!(Balances::free_balance(ACCOUNT_C), INIT_BALANCE + curr_amount);
        assert_eq!(
            last_event(),
            crate::Event::AssetTradedForCurrency(
                ASSET_A,
                ACCOUNT_B,
                ACCOUNT_C,
                curr_amount,
                token_amount,
            )
        );
    });
}

#[test]
fn asset_to_currency_fixed_output() {
    new_test_ext().execute_with(|| {
        let token_amount = 500;
        let curr_amount = 498; // token amount (500) - provider fee (0.3%) should be ~498

        assert_ok!(Dex::asset_to_currency(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_A,
            TradeAmount::FixedOutput {
                output_amount: curr_amount,
                max_input: token_amount
            },
            1,
            None
        ));

        let exchange = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, INIT_LIQUIDITY - curr_amount);
        assert_eq!(exchange.token_reserve, INIT_LIQUIDITY + token_amount);
        assert_eq!(Balances::free_balance(ACCOUNT_B), INIT_BALANCE + curr_amount);
        assert_eq!(Assets::maybe_balance(ASSET_A, ACCOUNT_B), Some(INIT_BALANCE - token_amount));
        let pallet_account = Test::pallet_account();
        assert_eq!(Balances::free_balance(pallet_account), INIT_LIQUIDITY - curr_amount);
        assert_eq!(
            Assets::maybe_balance(ASSET_A, pallet_account),
            Some(INIT_LIQUIDITY + token_amount)
        );
        assert_eq!(
            last_event(),
            crate::Event::AssetTradedForCurrency(
                ASSET_A,
                ACCOUNT_B,
                ACCOUNT_B,
                curr_amount,
                token_amount,
            )
        );
    });
}

#[test]
fn asset_to_asset_fixed_input() {
    new_test_ext().execute_with(|| {
        Dex::create_exchange(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_B,
            LIQ_TOKEN_B,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY,
        )
        .unwrap();

        let sold_token_amount = 500;
        let curr_amount = 498; // sold token amount (500) - provider fee (0.3%) should be ~498
        let bought_token_amount = 496; // currency amount (498) - provider fee (0.3%) should be ~496

        assert_ok!(Dex::asset_to_asset(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_A,
            ASSET_B,
            TradeAmount::FixedInput {
                input_amount: sold_token_amount,
                min_output: bought_token_amount,
            },
            1,
            None
        ));

        let exchange_a = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange_a.token_reserve, INIT_LIQUIDITY + sold_token_amount);
        assert_eq!(exchange_a.currency_reserve, INIT_LIQUIDITY - curr_amount);

        let exchange_b = Dex::exchanges(ASSET_B).unwrap();
        assert_eq!(exchange_b.token_reserve, INIT_LIQUIDITY - bought_token_amount);
        assert_eq!(exchange_b.currency_reserve, INIT_LIQUIDITY + curr_amount);

        assert_eq!(
            Assets::maybe_balance(ASSET_A, ACCOUNT_B),
            Some(INIT_BALANCE - sold_token_amount)
        );
        assert_eq!(
            Assets::maybe_balance(ASSET_B, ACCOUNT_B),
            Some(INIT_BALANCE + bought_token_amount)
        );

        let pallet_account = Test::pallet_account();
        assert_eq!(Balances::free_balance(pallet_account), INIT_LIQUIDITY + INIT_LIQUIDITY);
        assert_eq!(
            Assets::maybe_balance(ASSET_A, pallet_account),
            Some(INIT_LIQUIDITY + sold_token_amount)
        );
        assert_eq!(
            Assets::maybe_balance(ASSET_B, pallet_account),
            Some(INIT_LIQUIDITY - bought_token_amount)
        );

        assert_eq!(
            last_n_events(2),
            vec![
                crate::Event::AssetTradedForCurrency(
                    ASSET_A,
                    ACCOUNT_B,
                    pallet_account,
                    curr_amount,
                    sold_token_amount,
                ),
                crate::Event::CurrencyTradedForAsset(
                    ASSET_B,
                    pallet_account,
                    ACCOUNT_B,
                    curr_amount,
                    bought_token_amount,
                ),
            ]
        );
    });
}

#[test]
fn asset_to_asset_unsigned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::none(),
                ASSET_A,
                ASSET_B,
                TradeAmount::FixedInput {
                    input_amount: 1,
                    min_output: 1
                },
                1,
                None
            ),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn asset_to_asset_deadline_passed() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                ASSET_B,
                TradeAmount::FixedInput {
                    input_amount: 1,
                    min_output: 1
                },
                0,
                None
            ),
            crate::Error::<Test>::DeadlinePassed
        );
    });
}

#[test]
fn asset_to_asset_sold_token_amount_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                ASSET_B,
                TradeAmount::FixedInput {
                    input_amount: 0,
                    min_output: 1
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn asset_to_asset_min_bought_tokens_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                ASSET_B,
                TradeAmount::FixedInput {
                    input_amount: 1,
                    min_output: 0
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn asset_to_asset_max_sold_tokens_amount_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                ASSET_B,
                TradeAmount::FixedOutput {
                    output_amount: 1,
                    max_input: 0
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn asset_to_asset_output_bought_token_amount_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                ASSET_B,
                TradeAmount::FixedOutput {
                    output_amount: 0,
                    max_input: 1
                },
                1,
                None
            ),
            crate::Error::<Test>::TradeAmountIsZero
        );
    });
}

#[test]
fn asset_to_asset_not_enough_tokens() {
    new_test_ext().execute_with(|| {
        Dex::create_exchange(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_B,
            LIQ_TOKEN_B,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY,
        )
        .unwrap();

        let sold_token_amount = 500;
        // sold token amount (500) - provider fee (0.3%) should be ~498
        let bought_token_amount = 496; // currency amount (498) - provider fee (0.3%) should be ~496

        <Test as crate::Config>::Assets::burn_from(ASSET_A, &ACCOUNT_B, INIT_BALANCE).unwrap();

        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                ASSET_B,
                TradeAmount::FixedInput {
                    input_amount: sold_token_amount,
                    min_output: bought_token_amount,
                },
                1,
                None
            ),
            crate::Error::<Test>::NotEnoughTokens
        );
    });
}

#[test]
fn asset_to_asset_sold_asset_exchange_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_B,
                ASSET_A,
                TradeAmount::FixedInput {
                    input_amount: 1,
                    min_output: 1
                },
                1,
                None
            ),
            crate::Error::<Test>::ExchangeNotFound
        );
    });
}

#[test]
fn asset_to_asset_bought_asset_exchange_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                ASSET_B,
                TradeAmount::FixedInput {
                    input_amount: 1,
                    min_output: 1
                },
                1,
                None
            ),
            crate::Error::<Test>::ExchangeNotFound
        );
    });
}

#[test]
fn asset_to_asset_min_bought_tokens_too_high() {
    new_test_ext().execute_with(|| {
        Dex::create_exchange(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_B,
            LIQ_TOKEN_B,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY,
        )
        .unwrap();
        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                ASSET_B,
                TradeAmount::FixedInput {
                    input_amount: 10,
                    min_output: 50
                },
                1,
                None
            ),
            crate::Error::<Test>::MinBoughtTokensTooHigh
        );
    });
}

#[test]
fn asset_to_asset_max_sold_tokens_too_low() {
    new_test_ext().execute_with(|| {
        Dex::create_exchange(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_B,
            LIQ_TOKEN_B,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY,
        )
        .unwrap();
        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                ASSET_B,
                TradeAmount::FixedOutput {
                    output_amount: 50,
                    max_input: 10
                },
                1,
                None
            ),
            crate::Error::<Test>::MaxSoldTokensTooLow
        );
    });
}

#[test]
fn asset_to_asset_not_enough_liquidity() {
    new_test_ext().execute_with(|| {
        Dex::create_exchange(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_B,
            LIQ_TOKEN_B,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY,
        )
        .unwrap();
        assert_noop!(
            Dex::asset_to_asset(
                RuntimeOrigin::signed(ACCOUNT_B),
                ASSET_A,
                ASSET_B,
                TradeAmount::FixedOutput {
                    output_amount: INIT_LIQUIDITY + 1000,
                    max_input: INIT_LIQUIDITY + 1000
                },
                1,
                None
            ),
            crate::Error::<Test>::NotEnoughLiquidity
        );
    });
}

#[test]
fn asset_to_asset_transfer() {
    new_test_ext().execute_with(|| {
        Dex::create_exchange(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_B,
            LIQ_TOKEN_B,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY,
        )
        .unwrap();

        let sold_token_amount = 500;
        let curr_amount = 498; // sold token amount (500) - provider fee (0.3%) should be ~498
        let bought_token_amount = 496; // currency amount (498) - provider fee (0.3%) should be ~496

        assert_ok!(Dex::asset_to_asset(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_A,
            ASSET_B,
            TradeAmount::FixedInput {
                input_amount: sold_token_amount,
                min_output: bought_token_amount
            },
            1,
            Some(ACCOUNT_C)
        ));

        assert_eq!(
            Assets::maybe_balance(ASSET_A, ACCOUNT_B),
            Some(INIT_BALANCE - sold_token_amount)
        );
        assert_eq!(
            Assets::maybe_balance(ASSET_B, ACCOUNT_C),
            Some(INIT_BALANCE + bought_token_amount)
        );

        let pallet_account = Test::pallet_account();
        assert_eq!(
            last_n_events(2),
            vec![
                crate::Event::AssetTradedForCurrency(
                    ASSET_A,
                    ACCOUNT_B,
                    pallet_account,
                    curr_amount,
                    sold_token_amount,
                ),
                crate::Event::CurrencyTradedForAsset(
                    ASSET_B,
                    pallet_account,
                    ACCOUNT_C,
                    curr_amount,
                    bought_token_amount,
                ),
            ]
        );
    });
}

#[test]
fn asset_to_asset_fixed_output() {
    new_test_ext().execute_with(|| {
        Dex::create_exchange(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_B,
            LIQ_TOKEN_B,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY,
        )
        .unwrap();

        let sold_token_amount = 500;
        let curr_amount = 498; // sold token amount (500) - provider fee (0.3%) should be ~498
        let bought_token_amount = 496; // currency amount (498) - provider fee (0.3%) should be ~496

        assert_ok!(Dex::asset_to_asset(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_A,
            ASSET_B,
            TradeAmount::FixedOutput {
                output_amount: bought_token_amount,
                max_input: sold_token_amount
            },
            1,
            None
        ));

        let exchange_a = Dex::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange_a.token_reserve, INIT_LIQUIDITY + sold_token_amount);
        assert_eq!(exchange_a.currency_reserve, INIT_LIQUIDITY - curr_amount);

        let exchange_b = Dex::exchanges(ASSET_B).unwrap();
        assert_eq!(exchange_b.token_reserve, INIT_LIQUIDITY - bought_token_amount);
        assert_eq!(exchange_b.currency_reserve, INIT_LIQUIDITY + curr_amount);

        assert_eq!(
            Assets::maybe_balance(ASSET_A, ACCOUNT_B),
            Some(INIT_BALANCE - sold_token_amount)
        );
        assert_eq!(
            Assets::maybe_balance(ASSET_B, ACCOUNT_B),
            Some(INIT_BALANCE + bought_token_amount)
        );

        let pallet_account = Test::pallet_account();
        assert_eq!(Balances::free_balance(pallet_account), INIT_LIQUIDITY + INIT_LIQUIDITY);
        assert_eq!(
            Assets::maybe_balance(ASSET_A, pallet_account),
            Some(INIT_LIQUIDITY + sold_token_amount)
        );
        assert_eq!(
            Assets::maybe_balance(ASSET_B, pallet_account),
            Some(INIT_LIQUIDITY - bought_token_amount)
        );

        assert_eq!(
            last_n_events(2),
            vec![
                crate::Event::AssetTradedForCurrency(
                    ASSET_A,
                    ACCOUNT_B,
                    pallet_account,
                    curr_amount,
                    sold_token_amount,
                ),
                crate::Event::CurrencyTradedForAsset(
                    ASSET_B,
                    pallet_account,
                    ACCOUNT_B,
                    curr_amount,
                    bought_token_amount,
                ),
            ]
        );
    });
}

#[test]
fn trade_assets_back_and_forth() {
    new_test_ext().execute_with(|| {
        Dex::create_exchange(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_B,
            LIQ_TOKEN_B,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY,
        )
        .unwrap();

        let sold_token_amount = 500;
        // sold token amount (500) - provider fee (0.3%) should be ~498
        let bought_token_amount = 496; // currency amount (498) - provider fee (0.3%) should be ~496

        // Trade back and forth A -> B -> A
        assert_ok!(Dex::asset_to_asset(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_A,
            ASSET_B,
            TradeAmount::FixedOutput {
                output_amount: bought_token_amount,
                max_input: sold_token_amount,
            },
            1,
            None
        ));
        assert_ok!(Dex::asset_to_asset(
            RuntimeOrigin::signed(ACCOUNT_B),
            ASSET_B,
            ASSET_A,
            TradeAmount::FixedOutput {
                output_amount: bought_token_amount,
                max_input: sold_token_amount,
            },
            1,
            None
        ));

        // Remove all liquidity
        assert_ok!(Dex::remove_liquidity(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_A,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY + 4,
            1,
        ));
        assert_ok!(Dex::remove_liquidity(
            RuntimeOrigin::signed(ACCOUNT_A),
            ASSET_B,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY,
            INIT_LIQUIDITY + 4,
            1,
        ));

        // Account A should have received 4 (500-496) of both tokens as tx fees from account B
        assert_eq!(Balances::free_balance(ACCOUNT_A), INIT_BALANCE);
        assert_eq!(Assets::maybe_balance(ASSET_A, ACCOUNT_A), Some(INIT_BALANCE + 4));
        assert_eq!(Assets::maybe_balance(ASSET_B, ACCOUNT_A), Some(INIT_BALANCE + 4));
        assert_eq!(Assets::maybe_balance(ASSET_A, ACCOUNT_B), Some(INIT_BALANCE - 4));
        assert_eq!(Assets::maybe_balance(ASSET_B, ACCOUNT_B), Some(INIT_BALANCE - 4));
    });
}
