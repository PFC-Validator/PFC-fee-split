# Astroport - lp - Staking

The Staking Contract contains the logic for LP Token staking and reward distribution. XXX tokens
allocated for as liquidity incentives are distributed to stakers of the 
Astroport pair LP token.

# Source
[Valkyrie/contracts/lp_staking feature/migrate_terra branch](https://github.com/valkyrieprotocol/contracts/commit/b5fcb666f17d7e291f40365756e50fc0d7b9bf54)

# how does it work?

## Instantiation
you will need:
1. an administrator - a wallet or a multi-sig usually
2. a **token** that this contract accumulates to send to people 
3. a **name** for this contract 
4. a token that people will deposit (**lp_token**) that is used to apportion the **token**'s deposited to.

this contract does *NOT* work with native tokens.
before sending reward tokens, you will need to have at least one person depositing a **lp_token** into the system.


## deposits/receipt of CW20s
the contract will accept any CW20. It checks the address, and if it is the **lp_token** it will use this 
to '**bond**', and use it to calculate  the amount of rewards, otherwise it will assume it is one of the rewards,
and call **recv_reward_token**

### bond / unbond

on bonding, it increases the total number of bonded tokens being held, and keeps track of when (block#) the individual 
changed their stake. we also do a 'claim' internally, and store that amount in the 'pending' table, and then we update
the 'current' reward total/token in your last-claimed

We could have just sent the rewards earned so far, but astroport didn't handle that, so we introduced the 'pending' table

### recv_reward_token / withdrawing

when receiving tokens, it keeps a tally of how many tokens it has received ratio'ed by the number of 'bonded' tokens 
stored.

so if you have 100 bonded tokens, and get 1000 reward tokens, the total amount for that token is incremented by 10.

on withdrawing, it looks for any pending claims, and then calculates what the 
total-rewards - last-claimed-total rewards is, multiplies it by the amount of bonded tokens you hold, and sends you that for each token

as CW20's usually are stored with 6 decimal points, and we hold ratios in Decimal, there might be a situation where we have underflow.
(i.e. if you send it 0.000001 worth of a token as a reward when you have 100 different people bonded. it may not withdraw correctly)

## TODO
decide if we only want wallets, or should we allow contracts to use this?

