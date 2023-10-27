# WARNING

This has not been fully audited.
Do not use it.

# PFC Dust collector - KUJI edition

send any token (not CW20) into the contract. (if you're on the whitelist)
once a particular token hits a certain (configurable) amount, execute a swap via the defined stages. 
if the token is the 'collection' token, and is about a certain amount, it won't swap it, it will send it back to the source (steak currently)


## how?

It uses Mantaswap, and the swap strategy can be changed on a regular interval 

# TODO

+ setup so it actually calls a message for a given contract.
+ CW20 disbursement


# Thank you
PFC is always on the lookout for delegations, and new chains to validate on.
