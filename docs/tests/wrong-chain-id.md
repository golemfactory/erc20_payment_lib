## Wrong chain id test

### Source file

[wrong_chain_id.rs](../../tests/docker_02_errors/wrong_chain_id.rs)

### Description

Test is performing library behaviour on unrecoverable errors.

### Setup

- Common test [setup](./common-test-setup.md) is used
- Simple Geth without limits
- Simple RPC proxy without any limits

### What is tested:

- How to handle irrecoverable errors

### Notes:

-- TODO: right now library is stopping without proper error handling