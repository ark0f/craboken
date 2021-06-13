use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub minter: HumanAddr,
    pub total_supply: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Transfer {
        to: HumanAddr,
        amount: Uint128,
    },
    Burn {
        amount: Uint128,
    },
    SetAllowance {
        spender: HumanAddr,
        amount: Uint128,
        is_allowed: bool,
    },
    TransferFrom {
        from: HumanAddr,
        to: HumanAddr,
        amount: Uint128,
    },
    BurnFrom {
        from: HumanAddr,
        amount: Uint128,
    },
    Mint {
        recipient: HumanAddr,
        amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBalance { user: HumanAddr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BalanceResponse {
    pub amount: Uint128,
}
