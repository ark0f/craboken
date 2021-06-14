use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, Querier, StdError, StdResult, Storage, Uint128,
};

use crate::msg::{BalanceResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{Allowance, Allowances, Balances, ReadOnlyBalances, State};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        minter: msg.minter,
        total_supply: msg.total_supply,
    };

    State::write(&mut deps.storage).save(&state)?;

    debug_print!("Contract was initialized by {}", env.message.sender);

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Transfer { to, amount } => try_transfer(deps, env, to, amount),
        HandleMsg::Burn { amount } => try_burn(deps, env, amount),
        HandleMsg::SetAllowance {
            spender,
            amount,
            is_allowed,
        } => try_set_allowance(deps, env, spender, amount, is_allowed),
        HandleMsg::TransferFrom { from, to, amount } => {
            try_transfer_from(deps, env, from, to, amount)
        }
        HandleMsg::BurnFrom { from, amount } => try_burn_from(deps, env, from, amount),
        HandleMsg::Mint { recipient, amount } => try_mint(deps, env, recipient, amount),
    }
}

fn try_transfer<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    to: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;
    let to = deps.api.canonical_address(&to)?;
    try_transfer_inner(deps, sender_addr, to, amount)?;
    Ok(HandleResponse::default())
}

fn try_burn<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;
    try_burn_inner(deps, sender_addr, amount)?;
    Ok(HandleResponse::default())
}

fn try_set_allowance<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: HumanAddr,
    amount: Uint128,
    is_allowed: bool,
) -> StdResult<HandleResponse> {
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;
    let spender = deps.api.canonical_address(&spender)?;

    let mut allowances = Allowances::new(&sender_addr, &mut deps.storage);
    allowances.set(&spender, Allowance { is_allowed, amount })?;
    Ok(HandleResponse::default())
}

fn try_transfer_from<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    from: HumanAddr,
    to: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;
    let from = deps.api.canonical_address(&from)?;
    let to = deps.api.canonical_address(&to)?;

    process_allowance(&mut deps.storage, &from, &sender_addr, amount)?;

    try_transfer_inner(deps, from, to, amount)?;

    Ok(HandleResponse::default())
}

fn try_burn_from<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    from: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;
    let from = deps.api.canonical_address(&from)?;

    process_allowance(&mut deps.storage, &from, &sender_addr, amount)?;

    try_burn_inner(deps, from, amount)?;

    Ok(HandleResponse::default())
}

fn try_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    recipient: HumanAddr,
    Uint128(amount): Uint128,
) -> StdResult<HandleResponse> {
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;
    let recipient = deps.api.canonical_address(&recipient)?;

    let state = State::read(&deps.storage).load()?;
    let minter = deps.api.canonical_address(&state.minter)?;

    if minter != sender_addr {
        return Err(StdError::unauthorized());
    }

    let mut balances = Balances::new(&mut deps.storage);
    let recipient_balance = balances.get(&recipient)?;
    let new_recipient_balance = recipient_balance
        .checked_add(amount)
        .ok_or_else(|| StdError::generic_err("Too many tokens to mint for user"))?;
    balances.set(&recipient, new_recipient_balance)?;

    State::write(&mut deps.storage).update(|mut state| {
        state.total_supply = state
            .total_supply
            .u128()
            .checked_add(amount)
            .map(Uint128)
            .ok_or_else(|| {
                StdError::generic_err(
                    "More token are tried to create than available in total supply",
                )
            })?;
        Ok(state)
    })?;

    Ok(HandleResponse::default())
}

fn try_transfer_inner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    from: CanonicalAddr,
    to: CanonicalAddr,
    Uint128(amount): Uint128,
) -> StdResult<()> {
    let mut balances = Balances::new(&mut deps.storage);

    let sender_balance = balances.get(&from)?;
    let sender_new_balance = sender_balance
        .checked_sub(amount)
        .ok_or_else(|| StdError::generic_err("Too many tokens to transfer"))?;

    let to_balance = balances.get(&to)?;
    let recipient_new_balance = to_balance
        .checked_add(amount)
        .ok_or_else(|| StdError::generic_err("Too many tokens to receive"))?;

    balances.set(&from, sender_new_balance)?;
    balances.set(&to, recipient_new_balance)?;

    Ok(())
}

fn try_burn_inner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    from: CanonicalAddr,
    Uint128(amount): Uint128,
) -> StdResult<()> {
    let mut balances = Balances::new(&mut deps.storage);

    let sender_balance = balances.get(&from)?;
    let sender_new_balance = sender_balance
        .checked_sub(amount)
        .ok_or_else(|| StdError::generic_err("Too many tokens to burn"))?;
    balances.set(&from, sender_new_balance)?;

    State::write(&mut deps.storage).update(|mut state| {
        state.total_supply = state
            .total_supply
            .u128()
            .checked_sub(amount)
            .map(Uint128)
            .ok_or_else(|| {
                StdError::generic_err(
                    "More tokens are tried to burn than available in total supply",
                )
            })?;
        Ok(state)
    })?;

    Ok(())
}

fn process_allowance<S: Storage>(
    storage: &mut S,
    owner_addr: &CanonicalAddr,
    allowed_addr: &CanonicalAddr,
    amount: Uint128,
) -> StdResult<()> {
    let mut allowances = Allowances::new(owner_addr, storage);
    let mut allowance = allowances
        .get(allowed_addr)?
        .filter(|allowance| allowance.is_allowed)
        .ok_or_else(StdError::unauthorized)?;

    allowance.amount = allowance
        .amount
        .u128()
        .checked_sub(amount.u128())
        .map(Uint128)
        .ok_or_else(|| {
            StdError::generic_err("Amount of tokens is bigger than allowed to transfer")
        })?;

    allowances.set(allowed_addr, allowance)?;

    Ok(())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBalance { user } => to_binary(&query_balance(deps, user)?),
    }
}

fn query_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    user: HumanAddr,
) -> StdResult<BalanceResponse> {
    let user = deps.api.canonical_address(&user)?;

    let balances = ReadOnlyBalances::new(&deps.storage);
    let balance = balances.get(&user)?;
    Ok(BalanceResponse {
        amount: Uint128(balance),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ReadOnlyAllowances;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    const INITIAL_TOTAL_SUPPLY: u128 = 100_000_000;
    const INITIAL_BALANCE: u128 = 1_000_000;
    const ALLOWANCE_AMOUNT: u128 = 10_000;
    const TOTAL_SUPPLY: u128 = INITIAL_TOTAL_SUPPLY + INITIAL_BALANCE;

    fn init_contract<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>) {
        let msg = InitMsg {
            minter: "minter".into(),
            total_supply: Uint128(INITIAL_TOTAL_SUPPLY),
        };

        let env = mock_env("creator", &[]);

        let _res = init(deps, env, msg).unwrap();
    }

    fn mint<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>) {
        let msg = HandleMsg::Mint {
            recipient: "sender".into(),
            amount: Uint128(INITIAL_BALANCE),
        };

        let env = mock_env("minter", &[]);

        handle(deps, env, msg).unwrap();
    }

    fn set_allowance<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>) {
        let msg = HandleMsg::SetAllowance {
            spender: "third_party".into(),
            amount: Uint128(ALLOWANCE_AMOUNT),
            is_allowed: true,
        };

        let env = mock_env("sender", &[]);

        handle(deps, env, msg).unwrap();
    }

    #[test]
    fn proper_init() {
        let mut deps = mock_dependencies(16, &[]);
        init_contract(&mut deps);
    }

    #[test]
    fn handle_mint() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);
        mint(&mut deps);

        let state = State::read(&deps.storage).load().unwrap();
        assert_eq!(state.total_supply.u128(), TOTAL_SUPPLY);
    }

    #[test]
    fn handle_mint_unauthorized() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);

        let msg = HandleMsg::Mint {
            recipient: "sender".into(),
            amount: Uint128(1000),
        };

        let env = mock_env("not_minter", &[]);

        let err = handle(&mut deps, env, msg).unwrap_err();
        assert_eq!(err, StdError::unauthorized());
    }

    #[test]
    fn handle_mint_too_many() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);

        let msg = HandleMsg::Mint {
            recipient: "sender".into(),
            amount: Uint128(u128::MAX),
        };

        let env = mock_env("minter", &[]);

        handle(&mut deps, env, msg).unwrap_err();
    }

    #[test]
    fn handle_transfer() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);
        mint(&mut deps);

        let sender_env = mock_env("sender", &[]);

        let msg = HandleMsg::Transfer {
            to: "recipient".into(),
            amount: Uint128(1000),
        };

        handle(&mut deps, sender_env, msg).unwrap();

        let sender = deps
            .api
            .canonical_address(&HumanAddr::from("sender"))
            .unwrap();
        let recipient = deps
            .api
            .canonical_address(&HumanAddr::from("recipient"))
            .unwrap();

        let balances = ReadOnlyBalances::new(&deps.storage);

        let sender_balance = balances.get(&sender).unwrap();
        assert_eq!(sender_balance, INITIAL_BALANCE - 1000);

        let recipient_balance = balances.get(&recipient).unwrap();
        assert_eq!(recipient_balance, 1000);
    }

    #[test]
    fn handle_burn() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);
        mint(&mut deps);

        let sender_env = mock_env("sender", &[]);

        let msg = HandleMsg::Burn {
            amount: Uint128(1000),
        };

        handle(&mut deps, sender_env, msg).unwrap();

        let balances = ReadOnlyBalances::new(&deps.storage);

        let sender = deps
            .api
            .canonical_address(&HumanAddr::from("sender"))
            .unwrap();
        let sender_balance = balances.get(&sender).unwrap();
        assert_eq!(sender_balance, INITIAL_BALANCE - 1000);

        let state = State::read(&deps.storage).load().unwrap();
        assert_eq!(state.total_supply.u128(), TOTAL_SUPPLY - 1000);
    }

    #[test]
    fn handle_burn_more_than_total_supply() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);
        mint(&mut deps);

        State::write(&mut deps.storage)
            .update(|mut state| {
                state.total_supply = Uint128(0);
                Ok(state)
            })
            .unwrap();

        let sender_env = mock_env("sender", &[]);

        let msg = HandleMsg::Burn {
            amount: Uint128(1000),
        };

        handle(&mut deps, sender_env, msg).unwrap_err();
    }

    #[test]
    fn handle_set_allowance() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);
        mint(&mut deps);
        set_allowance(&mut deps);

        let owner = deps
            .api
            .canonical_address(&HumanAddr::from("sender"))
            .unwrap();
        let third_party = deps
            .api
            .canonical_address(&HumanAddr::from("third_party"))
            .unwrap();

        let allowances = ReadOnlyAllowances::new(&owner, &deps.storage);
        let allowance = allowances.get(&third_party).unwrap().unwrap();
        assert!(allowance.is_allowed);
        assert_eq!(allowance.amount.u128(), ALLOWANCE_AMOUNT);
    }

    #[test]
    fn handle_transfer_from() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);
        mint(&mut deps);
        set_allowance(&mut deps);

        let third_party_env = mock_env("third_party", &[]);

        let msg = HandleMsg::TransferFrom {
            from: "sender".into(),
            to: "recipient".into(),
            amount: Uint128(1000),
        };

        handle(&mut deps, third_party_env, msg).unwrap();

        let sender = deps
            .api
            .canonical_address(&HumanAddr::from("sender"))
            .unwrap();
        let recipient = deps
            .api
            .canonical_address(&HumanAddr::from("recipient"))
            .unwrap();
        let third_party = deps
            .api
            .canonical_address(&HumanAddr::from("third_party"))
            .unwrap();

        let balances = ReadOnlyBalances::new(&deps.storage);
        let recipient_balance = balances.get(&recipient).unwrap();
        assert_eq!(recipient_balance, 1000);

        let allowances = ReadOnlyAllowances::new(&sender, &deps.storage);
        let allowance = allowances.get(&third_party).unwrap().unwrap();
        assert_eq!(allowance.amount.u128(), ALLOWANCE_AMOUNT - 1000);
    }

    #[test]
    fn handle_transfer_from_too_many() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);
        mint(&mut deps);
        set_allowance(&mut deps);

        let third_party_env = mock_env("third_party", &[]);

        let msg = HandleMsg::TransferFrom {
            from: "sender".into(),
            to: "recipient".into(),
            amount: Uint128(ALLOWANCE_AMOUNT * 2),
        };

        handle(&mut deps, third_party_env, msg).unwrap_err();
    }

    #[test]
    fn handle_burn_from() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);
        mint(&mut deps);
        set_allowance(&mut deps);

        let third_party_env = mock_env("third_party", &[]);

        let msg = HandleMsg::BurnFrom {
            from: "sender".into(),
            amount: Uint128(1000),
        };

        handle(&mut deps, third_party_env, msg).unwrap();

        let sender = deps
            .api
            .canonical_address(&HumanAddr::from("sender"))
            .unwrap();
        let third_party = deps
            .api
            .canonical_address(&HumanAddr::from("third_party"))
            .unwrap();

        let balances = ReadOnlyBalances::new(&deps.storage);
        let sender_balance = balances.get(&sender).unwrap();
        assert_eq!(sender_balance, INITIAL_BALANCE - 1000);

        let allowances = ReadOnlyAllowances::new(&sender, &deps.storage);
        let allowance = allowances.get(&third_party).unwrap().unwrap();
        assert_eq!(allowance.amount.u128(), ALLOWANCE_AMOUNT - 1000);

        let state = State::read(&deps.storage).load().unwrap();
        assert_eq!(state.total_supply.u128(), TOTAL_SUPPLY - 1000);
    }

    #[test]
    fn handle_burn_from_too_many() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);
        mint(&mut deps);
        set_allowance(&mut deps);

        let third_party_env = mock_env("third_party", &[]);

        let msg = HandleMsg::BurnFrom {
            from: "sender".into(),
            amount: Uint128(ALLOWANCE_AMOUNT * 2),
        };

        handle(&mut deps, third_party_env, msg).unwrap_err();
    }

    #[test]
    fn query_get_balance() {
        let mut deps = mock_dependencies(16, &[]);

        init_contract(&mut deps);
        mint(&mut deps);

        let msg = QueryMsg::GetBalance {
            user: "sender".into(),
        };

        let resp = query(&mut deps, msg).unwrap();
        let resp: BalanceResponse = from_binary(&resp).unwrap();
        assert_eq!(resp.amount.u128(), INITIAL_BALANCE);
    }
}
