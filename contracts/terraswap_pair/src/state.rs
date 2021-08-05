use cosmwasm_std::Uint128;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::PairInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub pair_info: PairInfo,
    pub k_last: Uint128,
}

pub const CONFIG: Item<Config> = Item::new("config");
