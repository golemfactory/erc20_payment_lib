## Single gas transfer test

### Source file

[single_erc20_transfer.rs](../../tests/docker_01_single_transfer/single_erc20_transfer.rs)

### Description

Test is performing the simplest possible transfer of native (ETH or MATIC) token.

### Setup

 - Simple Geth without limits
 - Simple RPC proxy without any limits

### What is tested:

 - Runtime time lower than 30 secs
 - Checking number of events emitted
   - 1 Transfer event
   - 1 Transaction confirmed event
 - Check if gas_limit of transaction set to 21000
 - Checking if transfer was successful (check balance of receiver and sender)
 - Check if number of RPC calls is withing limits 10-40 (right now)

### Notes:
 - Expected runtime under 10 seconds
 - Expected fee paid: 0.000025336024875
 - No batching is used in this test.
 - No contract should be deployed.
 - No multi payment contract should be used.
