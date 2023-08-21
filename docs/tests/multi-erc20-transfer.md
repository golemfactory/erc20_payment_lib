## Multi erc20 token transfer test

### Source file

[docker_04_multi.rs](../../tests/docker_04_multi.rs)

### Description

Test is performing multiple transfers of ERC20 (tGLM) token.

### Setup

- Common test [setup](./common-test-setup.md) is used
- Simple Geth without limits
- Simple RPC proxy without any limits

### What is tested:

- Checking if multi-payment contract is correctly used
- Checking different methods of contract for validity. This is done by checking TransactionConfirmed message method field
- Checking if proper token amount is sent

### Notes:
- Running ERC20 transfer triggers approve contract event
- Batching to multiple receivers is used in this test.
- Multi payment contract is used.
