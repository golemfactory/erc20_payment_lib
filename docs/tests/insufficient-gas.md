## Single erc20 token transfer test

### Source file

[insufficinet_gas.rs](../../tests/docker_02_errors/insufficient_gas.rs)

### Description

Test is performing single payment.

### Setup

- Simple Geth without limits
- Simple RPC proxy without any limits

### What is tested:

- Behaviour of library when not enough is gas is on the account to perform next transactions
- Runtime is started with account with low ETH (not enough for single transaction)
- Check if event: TransactionStuck is emitted during the runtime

### Notes:

- How many times should be event emitted?
- 
