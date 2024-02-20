use cosmwasm_std::{Addr, Uint128};

use crate::{states::PendingClaimAmount, utils::merge_claims};

#[test]
fn merge() {
    let c = merge_claims(&[], &[]);
    assert!(c.is_empty(), "should be empty");
    let a: Vec<PendingClaimAmount> = vec![PendingClaimAmount {
        amount: Uint128::from(5u128),
        token: Addr::unchecked("test"),
    }];
    let c = merge_claims(&a, &[]);
    assert_eq!(a, c);
    let c = merge_claims(&[], &a);
    assert_eq!(a, c);
    let b: Vec<PendingClaimAmount> = vec![PendingClaimAmount {
        amount: Uint128::one(),
        token: Addr::unchecked("abc"),
    }];
    let mut c = merge_claims(&a, &b);
    c.sort_by(|a, b| a.token.cmp(&b.token));
    assert_eq!(
        c,
        vec![
            PendingClaimAmount {
                amount: Uint128::one(),
                token: Addr::unchecked("abc")
            },
            PendingClaimAmount {
                amount: Uint128::from(5u128),
                token: Addr::unchecked("test")
            },
        ]
    );
    let mut c = merge_claims(&b, &a);
    c.sort_by(|a, b| a.token.cmp(&b.token));
    assert_eq!(
        c,
        vec![
            PendingClaimAmount {
                amount: Uint128::one(),
                token: Addr::unchecked("abc")
            },
            PendingClaimAmount {
                amount: Uint128::from(5u128),
                token: Addr::unchecked("test")
            },
        ]
    );

    let one: Vec<PendingClaimAmount> = vec![PendingClaimAmount {
        amount: Uint128::from(12u128),
        token: Addr::unchecked("test"),
    }];
    let mut c = merge_claims(&a, &one);
    c.sort_by(|a, b| a.token.cmp(&b.token));
    assert_eq!(
        c,
        vec![PendingClaimAmount {
            amount: Uint128::from(17u128),
            token: Addr::unchecked("test")
        },]
    );
    let two: Vec<PendingClaimAmount> = vec![
        PendingClaimAmount {
            amount: Uint128::one(),
            token: Addr::unchecked("abc"),
        },
        PendingClaimAmount {
            amount: Uint128::from(12u128),
            token: Addr::unchecked("test"),
        },
    ];
    let mut c = merge_claims(&a, &two);
    c.sort_by(|a, b| a.token.cmp(&b.token));
    assert_eq!(
        c,
        vec![
            PendingClaimAmount {
                amount: Uint128::one(),
                token: Addr::unchecked("abc")
            },
            PendingClaimAmount {
                amount: Uint128::from(17u128),
                token: Addr::unchecked("test")
            },
        ]
    );
    let mut c = merge_claims(&two, &a);
    c.sort_by(|a, b| a.token.cmp(&b.token));
    assert_eq!(
        c,
        vec![
            PendingClaimAmount {
                amount: Uint128::one(),
                token: Addr::unchecked("abc")
            },
            PendingClaimAmount {
                amount: Uint128::from(17u128),
                token: Addr::unchecked("test")
            },
        ]
    );
}
