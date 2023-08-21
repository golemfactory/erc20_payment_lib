## Transfer stuck test

### Source file

[transfer_stuck.rs](../../tests/docker_02_errors/transfer_stuck.rs)

### Description

Test is performing single payment.

### Setup

- Common test [setup](./common-test-setup.md) is used
- Simple Geth without limits
- Simple RPC proxy without any limits

### What is tested:

- Single transaction is setup with very low gas price limit.
- Behaviour of library when gas price is set lower than network conditions
- Check if event TransactionStuck with reason TransactionStuckReason::GasPriceLow is emitted during runtime
- After some time library should finish transaction (during test network prices are getting lower)

### Notes:

- Detecting low gas price vs other problems (for example faulty node) may be tricky.
