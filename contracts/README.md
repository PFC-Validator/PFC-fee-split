# PFC-Fee-Splitter

## Table of contents

* [pfc-fee-splitter](#pfc-fee-splitter) [(source)](pfc-fee-splitter)
* [pfc-dust-collector-kujira](#pfc-dust-collector) [(source)](pfc-dust-collector-kujira)
* [pfc-vault](#pfc-vault) [(source)](pfc-vault-contract)
* [pfc-treasurechest](#pfc-treasurechest) [(source)](pfc-treasurechest-contract)

### pfc-fee-splitter

contract to take 'native' denoms and split the incoming native coins to various places.
it can also call a 'staking' contract to mint 'steak' style tokens and have them minted to an address.


### pfc-dust-collector
converts dust on Kuji chains to ukuji. 

### pfc-vault
Simple vault contract. 

### pfc-treasurechest
Simple vault contract. Uses a tokenfactoy as a 'ticket'. It only accepts a pool of tokens at the start 

## Deprecated
* [pfc-astroport-generator](#pfc-astroport-generator) [(source)](pfc-astroport-generator)


### pfc-astroport-generator


This to be used to handle astroport dual-reward incentives. you would send a CW20 of your choosing, and
stakers could bond their 'LP' token and receive rewards.

Inspiration
from [Valkyrie](https://github.com/valkyrieprotocol/contracts/tree/feature/migrate_terra2/contracts/lp_staking) & [GalacticDAO](https://github.com/galactic-dao/galactic-dao-contracts)
NFT Staking contracts
