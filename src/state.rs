use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    from_slice, to_vec, CanonicalAddr, HumanAddr, ReadonlyStorage, StdResult, Storage, Uint128,
};
use cosmwasm_storage::{
    singleton, singleton_read, PrefixedStorage, ReadonlyPrefixedStorage, ReadonlySingleton,
    Singleton,
};

const STATE_KEY: &[u8] = b"state";
const BALANCES_KEY: &[u8] = b"balances";
const ALLOWANCES_KEY: &[u8] = b"allowances";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct State {
    pub minter: HumanAddr,
    pub total_supply: Uint128,
}

impl State {
    pub fn write<S: Storage>(storage: &mut S) -> Singleton<S, Self> {
        singleton(storage, STATE_KEY)
    }

    pub fn read<S: Storage>(storage: &S) -> ReadonlySingleton<S, Self> {
        singleton_read(storage, STATE_KEY)
    }
}

pub struct Balances<'a, S: Storage> {
    storage: PrefixedStorage<'a, S>,
}

impl<'a, S: Storage> Balances<'a, S> {
    pub fn new(storage: &'a mut S) -> Self {
        let storage = PrefixedStorage::new(BALANCES_KEY, storage);
        Self { storage }
    }

    pub fn set(&mut self, addr: &CanonicalAddr, amount: u128) -> StdResult<()> {
        self.storage
            .set(addr.as_slice(), &to_vec(&Uint128(amount))?);
        Ok(())
    }

    pub fn get(&self, addr: &CanonicalAddr) -> StdResult<u128> {
        ReadOnlyBalancesImpl(&self.storage).get(addr)
    }
}

pub struct ReadOnlyBalances<'a, S: Storage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: Storage> ReadOnlyBalances<'a, S> {
    pub fn new(storage: &'a S) -> Self {
        let storage = ReadonlyPrefixedStorage::new(BALANCES_KEY, storage);
        Self { storage }
    }

    pub fn get(&self, addr: &CanonicalAddr) -> StdResult<u128> {
        ReadOnlyBalancesImpl(&self.storage).get(addr)
    }
}

struct ReadOnlyBalancesImpl<'a, S: ReadonlyStorage>(&'a S);

impl<'a, S: ReadonlyStorage> ReadOnlyBalancesImpl<'a, S> {
    fn get(&self, addr: &CanonicalAddr) -> StdResult<u128> {
        Ok(self
            .0
            .get(addr.as_slice())
            .as_deref()
            .map(from_slice)
            .transpose()?
            .map(|num: Uint128| num.u128())
            .unwrap_or(0))
    }
}

pub struct Allowances<'a, S: Storage> {
    storage: PrefixedStorage<'a, S>,
}

impl<'a, S: Storage> Allowances<'a, S> {
    pub fn new(owner: &CanonicalAddr, storage: &'a mut S) -> Self {
        let storage = PrefixedStorage::multilevel(&[ALLOWANCES_KEY, owner.as_slice()], storage);
        Self { storage }
    }

    pub fn set(&mut self, addr: &CanonicalAddr, allowance: Allowance) -> StdResult<()> {
        self.storage.set(addr.as_slice(), &to_vec(&allowance)?);
        Ok(())
    }

    pub fn get(&self, addr: &CanonicalAddr) -> StdResult<Option<Allowance>> {
        ReadOnlyAllowancesImpl(&self.storage).get(addr)
    }
}

pub struct ReadOnlyAllowances<'a, S: Storage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: Storage> ReadOnlyAllowances<'a, S> {
    pub fn new(owner: &CanonicalAddr, storage: &'a S) -> Self {
        let storage =
            ReadonlyPrefixedStorage::multilevel(&[ALLOWANCES_KEY, owner.as_slice()], storage);
        Self { storage }
    }

    pub fn get(&self, addr: &CanonicalAddr) -> StdResult<Option<Allowance>> {
        ReadOnlyAllowancesImpl(&self.storage).get(addr)
    }
}

struct ReadOnlyAllowancesImpl<'a, S: ReadonlyStorage>(&'a S);

impl<'a, S: ReadonlyStorage> ReadOnlyAllowancesImpl<'a, S> {
    fn get(&self, addr: &CanonicalAddr) -> StdResult<Option<Allowance>> {
        self.0
            .get(addr.as_slice())
            .as_deref()
            .map(from_slice)
            .transpose()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Allowance {
    pub is_allowed: bool,
    pub amount: Uint128,
}
