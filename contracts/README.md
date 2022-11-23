# PFC-Fee-Splitter

## Table of contents

* [pfc-fee-splitter](#pfc-fee-splitter) [(source)](pfc-fee-splitter)
* [pfc-astroport_lp_staker](#pfc-astroport_lp_staker) [(source)](pfc-astroport_lp_staker)

### pfc-fee-splitter

contract to take 'native' denoms and split the incoming native coins to various places.
it can also call a 'staking' contract to mint 'steak' style tokens and have them minted to an address.

### pfc-astroport_lp_staker

WIP.
This to be used to handle astroport dual-reward incentives. you would send a CW20 of your choosing, and
stakers could bond their 'LP' token and receive rewards.

Inspiration
from [Valkyrie](https://github.com/valkyrieprotocol/contracts/tree/feature/migrate_terra2/contracts/lp_staking) & [GalacticDAO](https://github.com/galactic-dao/galactic-dao-contracts)
NFT Staking contracts
