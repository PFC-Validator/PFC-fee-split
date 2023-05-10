# WARNING

This has not been fully audited.
Do not use it.

# PFC-FEE Split

Split a collection of native tokens into multiple wallets.

## How?

- money comes in via Deposit messages. When it is called, it splits the funds sent (native only)
  and sends them to the various allocation wallets based on the configured allocation ratios.
  Each allocation has a minimum amount (configurable) where it won't send until it reaches that threshold. You can
  bypass these thresholds by sending 'flush:true' if you are whitelisted.

- There is also a 'reconcile' function that will take existing funds sitting in the contract, and redistibute them based
  on allocation holdings. (note: It will ignore thresholds)

# TODO

+ setup so it actually calls a message for a given contract.
+ CW20 disbursement


# Thank you
The audit for this contract has been sponsored by [backbonelabs.io](https://backbonelabs.io).
