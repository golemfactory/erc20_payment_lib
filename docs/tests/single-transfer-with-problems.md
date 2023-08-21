## Single transfer with problems test

### Source file

[single_transfer_with_problems.rs](../../tests/docker_03_problems/single_transfer_with_problems.rs)

### Description

Test is performing single payment, but RPC is generating errors when connected.

### Setup

- Common test [setup](./common-test-setup.md) is used
- Simple Geth without limits
- Simple RPC proxy with additional error generation
- RPC proxy is setup using REST API - /api/problems/set/ endpoint of web3 proxy

### What is tested:

- This test is similar to single ERC20 transfer, but random errors are added to proxy
- Expected behaviour is that despite multiple error during rpc connections runtime will manage to proceed with transactions
- 

### Notes:

- Probably we should also check for transaction stuck event due to RPC errors?
