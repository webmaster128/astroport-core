use cosmwasm_std::{Addr, Decimal, Env, QuerierWrapper, Storage};
use injective_cosmwasm::InjectiveQueryWrapper;
use itertools::Itertools;

use astroport::asset::{Asset, DecimalAsset};
use astroport::cosmwasm_ext::IntegerToDecimal;
use astroport::observation::{safe_sma_buffer_not_full, safe_sma_calculation};
use astroport::observation::{Observation, PrecommitObservation};
use astroport_circular_buffer::error::BufferResult;
use astroport_circular_buffer::BufferManager;
use astroport_pcl_common::state::{Config, Precisions};

use crate::error::ContractError;
use crate::orderbook::state::OrderbookState;
use crate::orderbook::utils::get_subaccount_balances_dec;
use crate::state::OBSERVATIONS;

pub(crate) fn query_contract_balances(
    querier: QuerierWrapper<InjectiveQueryWrapper>,
    addr: &Addr,
    config: &Config,
    precisions: &Precisions,
) -> Result<Vec<DecimalAsset>, ContractError> {
    config
        .pair_info
        .query_pools(&querier, addr)?
        .into_iter()
        .map(|asset| {
            asset
                .to_decimal_asset(precisions.get_precision(&asset.info)?)
                .map_err(Into::into)
        })
        .collect()
}

/// Returns current pool's volumes where amount is in [`Decimal256`] form.
pub(crate) fn query_pools(
    querier: QuerierWrapper<InjectiveQueryWrapper>,
    addr: &Addr,
    config: &Config,
    ob_config: &OrderbookState,
    precisions: &Precisions,
    subacc_deposits: Option<&[Asset]>,
) -> Result<Vec<DecimalAsset>, ContractError> {
    let mut contract_assets = query_contract_balances(querier, addr, config, precisions)?;

    let ob_deposits = if let Some(ob_deposits) = subacc_deposits {
        ob_deposits
            .iter()
            .map(|asset| {
                asset
                    .amount
                    .to_decimal256(precisions.get_precision(&asset.info)?)
                    .map_err(Into::into)
            })
            .collect::<Result<Vec<_>, ContractError>>()?
    } else {
        let querier = injective_cosmwasm::InjectiveQuerier::new(&querier);
        get_subaccount_balances_dec(
            &config.pair_info.asset_infos,
            precisions,
            &querier,
            &ob_config.subaccount,
        )?
        .into_iter()
        .map(|asset| asset.amount)
        .collect_vec()
    };

    // merge real assets with orderbook deposits
    contract_assets
        .iter_mut()
        .zip(ob_deposits)
        .for_each(|(asset, deposit)| {
            asset.amount += deposit;
        });

    Ok(contract_assets)
}

/// Calculate and save price moving average
pub fn accumulate_swap_sizes(
    storage: &mut dyn Storage,
    env: &Env,
    ob_state: &mut OrderbookState,
) -> BufferResult<()> {
    if let Some(PrecommitObservation {
        base_amount,
        quote_amount,
        precommit_ts,
    }) = PrecommitObservation::may_load(storage)?
    {
        let mut buffer = BufferManager::new(storage, OBSERVATIONS)?;
        let observed_price = Decimal::from_ratio(base_amount, quote_amount);

        let new_observation;
        if let Some(last_obs) = buffer.read_last(storage)? {
            // Skip saving observation if it has been already saved
            if last_obs.ts < precommit_ts {
                // Since this is circular buffer the next index contains the oldest value
                let count = buffer.capacity();
                if let Some(oldest_obs) = buffer.read_single(storage, buffer.head() + 1)? {
                    let price_sma = safe_sma_calculation(
                        last_obs.price_sma,
                        oldest_obs.price,
                        count,
                        observed_price,
                    )?;
                    new_observation = Observation {
                        ts: precommit_ts,
                        price: observed_price,
                        price_sma,
                    };
                } else {
                    // Buffer is not full yet
                    let count = buffer.head();
                    let price_sma =
                        safe_sma_buffer_not_full(last_obs.price_sma, count, observed_price)?;
                    new_observation = Observation {
                        ts: precommit_ts,
                        price: observed_price,
                        price_sma,
                    };
                }

                // Enable orderbook if we have enough observations
                if !ob_state.ready && (buffer.head() + 1) >= ob_state.min_trades_to_avg {
                    ob_state.ready(true)
                }

                buffer.instant_push(storage, &new_observation)?
            }
        } else {
            // Buffer is empty
            if env.block.time.seconds() > precommit_ts {
                new_observation = Observation {
                    ts: precommit_ts,
                    price: observed_price,
                    price_sma: observed_price,
                };

                buffer.instant_push(storage, &new_observation)?
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_env, MockStorage};
    use cosmwasm_std::{BlockInfo, Timestamp};
    use injective_cosmwasm::{MarketId, SubaccountId};

    use crate::orderbook::consts::MIN_TRADES_TO_AVG_LIMITS;

    use super::*;

    fn next_block(block: &mut BlockInfo) {
        block.height += 1;
        block.time = block.time.plus_seconds(1);
    }

    #[test]
    fn test_swap_observations() {
        let mut store = MockStorage::new();
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(1);
        let mut ob_state = OrderbookState {
            market_id: MarketId::unchecked("test"),
            subaccount: SubaccountId::unchecked("test"),
            asset_infos: vec![],
            min_price_tick_size: Default::default(),
            min_quantity_tick_size: Default::default(),
            need_reconcile: false,
            last_balances: vec![],
            orders_number: 0,
            liquidity_percent: Default::default(),
            min_base_order_size: Default::default(),
            min_quote_order_size: Default::default(),
            min_trades_to_avg: *MIN_TRADES_TO_AVG_LIMITS.start(),
            ready: false,
            enabled: true,
        };
        BufferManager::init(&mut store, OBSERVATIONS, 10).unwrap();

        for _ in 0..=50 {
            accumulate_swap_sizes(&mut store, &env, &mut ob_state).unwrap();
            PrecommitObservation::save(&mut store, &env, 1000u128.into(), 500u128.into()).unwrap();
            next_block(&mut env.block);
        }

        let buffer = BufferManager::new(&store, OBSERVATIONS).unwrap();

        let obs = buffer.read_last(&store).unwrap().unwrap();
        assert_eq!(obs.ts, 50);
        assert_eq!(buffer.head(), 0);
        assert_eq!(obs.price, Decimal::raw(2));
        assert_eq!(obs.price_sma, Decimal::raw(2));
    }

    #[ignore]
    #[test]
    fn test_contract_ready() {
        let mut store = MockStorage::new();
        let mut env = mock_env();
        let min_trades_to_avg = 10;
        let mut ob_state = OrderbookState {
            market_id: MarketId::unchecked("test"),
            subaccount: SubaccountId::unchecked("test"),
            asset_infos: vec![],
            min_price_tick_size: Default::default(),
            min_quantity_tick_size: Default::default(),
            need_reconcile: false,
            last_balances: vec![],
            orders_number: 0,
            liquidity_percent: Default::default(),
            min_base_order_size: Default::default(),
            min_quote_order_size: Default::default(),
            min_trades_to_avg,
            ready: false,
            enabled: true,
        };
        BufferManager::init(&mut store, OBSERVATIONS, min_trades_to_avg).unwrap();

        for _ in 0..min_trades_to_avg {
            accumulate_swap_sizes(&mut store, &env, &mut ob_state).unwrap();
            PrecommitObservation::save(&mut store, &env, 1000u128.into(), 500u128.into()).unwrap();
            next_block(&mut env.block);
        }
        assert!(!ob_state.ready, "Contract should not be ready yet");

        // last observation to make contract ready
        accumulate_swap_sizes(&mut store, &env, &mut ob_state).unwrap();

        assert!(ob_state.ready, "Contract should be ready");
    }
}
