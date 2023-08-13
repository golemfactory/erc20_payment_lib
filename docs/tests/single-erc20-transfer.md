## Single erc20 token transfer test

### Source file

[single_erc20_transfer.rs](../../tests/docker_01_basic/single_erc20_transfer.rs)

### Description

Test is performing transfer of ERC20 (tGLM) token.

### Setup

 - Simple Geth without limits
 - Simple RPC proxy without any limits

### What is tested:

 - Runtime time lower than 30 secs
 - Checking number of events emitted
   - 1 Approve event
   - 1 Transfer event
   - 2 Transaction confirmed event
 - Check if gas_limit of approval set to 66572
 - Check if gas_limit of erc20 transfer set to 71482
 - Checking if transfer was successful (check balance of receiver and sender)
 - Check if number of RPC calls is withing limits 30-70 (right now)

### Notes:
 - Running ERC20 transfer triggers approve contract event
 - Expected runtime under 15 seconds
 - Expected fee paid: 0.000118535047714026
 - No batching is used in this test.
 - No multi payment contract should be used.
