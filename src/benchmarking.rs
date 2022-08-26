use crate::{AccountIdOf, Call, Config, Pallet, TradeAmount};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::pallet_prelude::DispatchResult;
use frame_support::traits::{
    fungibles::{Create, Mutate},
    Currency,
};
use frame_system::RawOrigin;

const INIT_BALANCE: u128 = 1_000_000_000_000_000;
const INIT_LIQUIDITY: u128 = 1_000_000_000_000;
const ASSET_A: u32 = 1;
const ASSET_B: u32 = 2;
const LIQ_TOKEN_A: u32 = 11;
const LIQ_TOKEN_B: u32 = 12;

fn prepare_exchange<T>(asset_id: u32, liquidity_token_id: u32) -> DispatchResult
where
    T: frame_system::Config<BlockNumber = u32>,
    T: Config<AssetId = u32, AssetBalance = u128>,
    T::Currency: Currency<AccountIdOf<T>, Balance = u128>,
    T::Assets: Create<AccountIdOf<T>> + Mutate<AccountIdOf<T>>,
{
    let caller: T::AccountId = whitelisted_caller();
    T::Assets::create(asset_id, caller.clone(), true, 1)?;
    T::Assets::mint_into(asset_id, &caller, INIT_BALANCE)?;
    T::Currency::make_free_balance_be(&caller, INIT_BALANCE);
    Pallet::<T>::create_exchange(
        RawOrigin::Signed(caller.clone()).into(),
        asset_id,
        liquidity_token_id,
        INIT_LIQUIDITY,
        INIT_LIQUIDITY,
    )?;
    Ok(())
}

benchmarks! {
    where_clause {
        where
            T: frame_system::Config<BlockNumber = u32>,
            T: Config<AssetId = u32, AssetBalance = u128>,
            T::Currency: Currency<AccountIdOf<T>, Balance = u128>,
            T::Assets: Create<AccountIdOf<T>> + Mutate<AccountIdOf<T>>,
    }

    create_exchange {
        let caller: T::AccountId = whitelisted_caller();
        T::Assets::create(ASSET_B, caller.clone(), true, 1).unwrap();
        T::Assets::mint_into(ASSET_B, &caller, INIT_BALANCE).unwrap();
        T::Currency::make_free_balance_be(&caller, INIT_BALANCE);
    }: _(RawOrigin::Signed(caller), ASSET_B, LIQ_TOKEN_B, INIT_LIQUIDITY, INIT_LIQUIDITY)
    verify {
        assert!(Pallet::<T>::exchanges(ASSET_B).is_some());
    }

    add_liquidity {
        prepare_exchange::<T>(ASSET_A, LIQ_TOKEN_A)?;
        let caller: T::AccountId = whitelisted_caller();
        // Token amount is 2, not 1 because of the `+1` in liquidity added formula
    }: _(RawOrigin::Signed(caller), ASSET_A, 1, 1, 2, 1)
    verify {
        let exchange = Pallet::<T>::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, INIT_LIQUIDITY + 1);
        assert_eq!(exchange.token_reserve, INIT_LIQUIDITY + 2);
    }

    remove_liquidity {
        prepare_exchange::<T>(ASSET_A, LIQ_TOKEN_A)?;
        let caller: T::AccountId = whitelisted_caller();
    }: _(RawOrigin::Signed(caller), ASSET_A, 1, 1, 1, 1)
    verify {
        let exchange = Pallet::<T>::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, INIT_LIQUIDITY - 1);
        assert_eq!(exchange.token_reserve, INIT_LIQUIDITY - 1);
    }

    currency_to_asset {
        prepare_exchange::<T>(ASSET_A, LIQ_TOKEN_A)?;
        let caller: T::AccountId = whitelisted_caller();
        let input_amount = 500;
        let min_output = 498; // sold amount (500) - provider fee (0.3%) should be ~498
    }: _(RawOrigin::Signed(caller), ASSET_A, TradeAmount::FixedInput{input_amount, min_output}, 1, None)
    verify {
        let exchange = Pallet::<T>::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, INIT_LIQUIDITY + input_amount);
        assert_eq!(exchange.token_reserve, INIT_LIQUIDITY - min_output);
    }

    asset_to_currency {
        prepare_exchange::<T>(ASSET_A, LIQ_TOKEN_A)?;
        let caller: T::AccountId = whitelisted_caller();
        let input_amount = 500;
        let min_output = 498; // sold amount (500) - provider fee (0.3%) should be ~498
    }: _(RawOrigin::Signed(caller), ASSET_A, TradeAmount::FixedInput{input_amount, min_output}, 1, None)
    verify {
        let exchange = Pallet::<T>::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange.currency_reserve, INIT_LIQUIDITY - min_output);
        assert_eq!(exchange.token_reserve, INIT_LIQUIDITY + input_amount);
    }

    asset_to_asset {
        prepare_exchange::<T>(ASSET_A, LIQ_TOKEN_A)?;
        prepare_exchange::<T>(ASSET_B, LIQ_TOKEN_B)?;
        let caller: T::AccountId = whitelisted_caller();
        let input_amount = 500;
        let currency_amount = 498; // sold amount (500) - provider fee (0.3%) should be ~498
        let min_output = 496; // currency amount (498) - provider fee (0.3%) should be ~496
    }: _(RawOrigin::Signed(caller), ASSET_A, ASSET_B, TradeAmount::FixedInput{input_amount, min_output}, 1, None)
    verify {
        let exchange_a = Pallet::<T>::exchanges(ASSET_A).unwrap();
        assert_eq!(exchange_a.currency_reserve, INIT_LIQUIDITY - currency_amount);
        assert_eq!(exchange_a.token_reserve, INIT_LIQUIDITY + input_amount);

        let exchange_b = Pallet::<T>::exchanges(ASSET_B).unwrap();
        assert_eq!(exchange_b.currency_reserve, INIT_LIQUIDITY + currency_amount);
        assert_eq!(exchange_b.token_reserve, INIT_LIQUIDITY - min_output);
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
