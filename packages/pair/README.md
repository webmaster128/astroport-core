# Astroport: Pair Interface

This is a collection of types and queriers which are commonly used with Astroport pair contracts.

---

## InstantiateMsg

Initializes a new x*y=k pair.

```json
{
  "token_code_id": 123,
  "factory_addr": "terra...",
  "asset_infos": [
    {
      "token": {
        "contract_addr": "terra..."
      }
    },
    {
      "native_token": {
        "denom": "uusd"
      }
    }
  ],
  "init_params": "<base64_encoded_json_string: optional binary serialised parameters for custom pool types>"
}
```

## ExecuteMsg

### `receive`

Withdraws liquidity or assets that were swapped to (ask assets in a swap operation).

```json
{
  "receive": {
    "sender": "terra...",
    "amount": "123",
    "msg": "<base64_encoded_json_string>"
  }
}
```

### `provide_liquidity`

Provides liquidity by sending a user's native or token assets to the pool.

__NOTE__: you should increase your token allowance for the pool before providing liquidity!

1. Providing Liquidity Without Specifying Slippage Tolerance

```json
  {
    "provide_liquidity": {
      "assets": [
        {
          "info": {
            "token": {
              "contract_addr": "terra..."
            }
          },
          "amount": "1000000"
        },
        {
          "info": {
            "native_token": {
              "denom": "uusd"
            }
          },
          "amount": "1000000"
        }
      ],
      "auto_stake": false,
      "receiver": "terra..."
    }
  }
```

2. Providing Liquidity With Slippage Tolerance

  ```json
  {
    "provide_liquidity": {
      "assets": [
        {
          "info": {
            "token": {
              "contract_addr": "terra..."
            }
          },
          "amount": "1000000"
        },
        {
          "info": {
            "native_token": {
              "denom": "uusd"
            }
          },
          "amount": "1000000"
        }
      ],
      "slippage_tolerance": "0.01",
      "auto_stake": false,
      "receiver": "terra..."
    }
  }
```

### `withdraw_liquidity`

Burn LP tokens and withdraw liquidity from a pool. This call must be sent to a LP token contract associated with the pool from which you want to withdraw liquidity from.

```json
  {
    "withdraw_liquidity": {}
  }
```

### `swap`

Perform a swap. `offer_asset` is your source asset and `to` is the address that will receive the ask assets. All fields are optional except `offer_asset`.

NOTE: You should increase token allowance before swap.

```json
  {
    "swap": {
      "offer_asset": {
        "info": {
          "native_token": {
            "denom": "uluna"
          }
        },
        "amount": "123"
      },
      "belief_price": "123",
      "max_spread": "123",
      "to": "terra..."
    }
  }
```

### `update_config`

The contract configuration cannot be updated.

```json
  {
    "update_config": {
      "params": "<base64_encoded_json_string>"
    }
  }
```

## QueryMsg

All query messages are described below. A custom struct is defined for each query response.

### `pair`

Retrieve a pair's configuration (type, assets traded in it etc)

```json
{
  "pair": {}
}
```

### `pool`

Returns the amount of tokens in the pool for all assets as well as the amount of LP tokens issued.

```json
{
  "pool": {}
}
```

### `config`

Get the pair contract configuration.

```json
{
  "config": {}
}
```

### `share`

Return the amount of assets someone would get from the pool if they were to burn a specific amount of LP tokens.

```json
{
  "share": {
    "amount": "123"
  }
}
```

### `simulation`

Simulates a swap and returns the spread and commission amounts.

```json
{
  "simulation": {
    "offer_asset": {
      "info": {
        "native_token": {
          "denom": "uusd"
        }
      },
      "amount": "1000000"
    }
  }
}
```

### `reverse_simulation`

Reverse simulates a swap (specifies the ask instead of the offer) and returns the offer amount, spread and commission.

```json
{
  "reverse_simulation": {
    "ask_asset": {
      "info": {
        "token": {
          "contract_addr": "terra..."
        }
      },
      "amount": "1000000"
    }
  }
}
```

### `cumulative_prices`

Returns the cumulative prices for the assets in the pair.

```json
{
  "cumulative_prices": {}
}
```

## Data Types

### PairInfo

This structure stores the main parameters for an Astroport pair.

```rust
pub struct PairInfo {
    pub asset_infos: [AssetInfo; 2],
    pub contract_addr: Addr,
    pub liquidity_token: Addr,
    pub pair_type: PairType,
}
```

## Queriers

## Swap Pairs Simulating

### Simulate

Simulates a swap and returns the output amount, the spread and commission amounts.

```rust
pub fn simulate(
    querier: &QuerierWrapper,
    pair_contract: impl Into<String>,
    offer_asset: &Asset,
    ask_asset_info: Option<AssetInfo>,
) -> StdResult<SimulationResponse>
```

### Reverse Simulate

Simulates a reverse swap and returns an input amount, the spread and commission amounts.

```rust
pub fn reverse_simulate(
    querier: &QuerierWrapper,
    pair_contract: impl Into<String>,
    ask_asset: &Asset,
    offer_asset_info: Option<AssetInfo>,
) -> StdResult<ReverseSimulationResponse>
```