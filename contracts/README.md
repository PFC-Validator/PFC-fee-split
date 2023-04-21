# PFC-Fee-Splitter

## Table of contents

* [pfc-fee-splitter](#pfc-fee-splitter) [(source)](pfc-fee-splitter)
* [pfc-astroport-generator](#pfc-astroport-generator) [(source)](pfc-astroport-generator)
* [pfc-vault](#pfc-vault) [(source)](pfc-vault-contract)

### pfc-fee-splitter

contract to take 'native' denoms and split the incoming native coins to various places.
it can also call a 'staking' contract to mint 'steak' style tokens and have them minted to an address.

### pfc-astroport-generator

WIP.
This to be used to handle astroport dual-reward incentives. you would send a CW20 of your choosing, and
stakers could bond their 'LP' token and receive rewards.

Inspiration
from [Valkyrie](https://github.com/valkyrieprotocol/contracts/tree/feature/migrate_terra2/contracts/lp_staking) & [GalacticDAO](https://github.com/galactic-dao/galactic-dao-contracts)
NFT Staking contracts

### pfc-vault
Simple vault contract.

# how do/can they interact ?

![](../images/overview.png "overview")

native denoms comes into the system via 'fee-splitter' contract. the funds get split via the allocation table
and either TRANSFERed to a wallet, or SEND to a smart contract, in this case, the 'vault' contract.

the vault accumulates the denoms, and allows people who hold the governance token (in astroport's case the LP token of the pair) to claim their portion when they choose.

the person needs to deposit their governance token in to start accumulating rewards.

the astroport-generator sits between the end-user and the vault itself, so the vault only sees a single address (the astroport smart contract) as the holder of it's LP tokens.
it forwards the 'claim' function from astroport, and send rewards (in total for the smart contract).
Astrport then determines how much of that sends to the individual. (how is out of scope)