use crate::states::PendingClaimAmount;
use cosmwasm_std::{Addr, Uint128};
use std::collections::HashMap;

pub fn merge_claims(
    a: &Vec<PendingClaimAmount>,
    b: &Vec<PendingClaimAmount>,
) -> Vec<PendingClaimAmount> {
    if a.is_empty() {
        return b.clone();
    }
    if b.is_empty() {
        return a.clone();
    }
    let mut a_map = a
        .iter()
        .map(|p| (p.token.clone(), p.amount))
        .collect::<HashMap<Addr, Uint128>>();
    let b_map = b
        .iter()
        .map(|p| (p.token.clone(), p.amount))
        .collect::<HashMap<Addr, Uint128>>();

    for b_entry in b_map {
        a_map
            .entry(b_entry.0)
            .and_modify(|e| *e += b_entry.1)
            .or_insert(b_entry.1);
    }

    a_map
        .into_iter()
        .map(|addr| PendingClaimAmount {
            amount: addr.1,
            token: addr.0,
        })
        .collect()
}
