## Custom test

### Source file

[custom_long.rs](../../tests/custom_long.rs)

### Description

Customizable test base

### Setup

- Common test [setup](./common-test-setup.md) is used
- Simple Geth without limits
- Simple RPC proxy without limits

### What is tested:

- Behaviour of runtime in more "normal" condition.
- Transfers are generated from the stream during operations.
- Multiple ENV variables to control test.
- Not much validity checks are done, rather stability and performance of the library is checked.
- Stats API is used to check if all transfers are finished (and not lost) 

### Notes:

- This is right now most customizable test and other features will be added