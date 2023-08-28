## Custom test base

### Source file

[custom_long.rs](../../tests/custom_long.rs)

### Description

Customizable test base for checking performance/robustness

### Setup

- Common test [setup](./common-test-setup.md) is used
- Simple Geth without limits
- Simple RPC proxy without limits

### Custom github action
- To run this tests on gihub actions you can use [custom_10x action](https://github.com/golemfactory/erc20_payment_lib/actions/workflows/custom_10x.yml) and click run_workflow directly from github.
- DB files from tests are uploaded so you can inspect them further for more info about runtime run.
- By default wal option is used in sqlite so in order to have one file library command cleanup is used.

### What is tested:
- Behaviour of runtime in more "normal" condition.
- Transfers are generated from the stream during operations.
- Multiple ENV variables to control test.
- Not much validity checks are done, rather stability and performance of the library is checked.
- Stats API is used to check if all transfers are finished (and not lost) 

### Notes:

- This is right now most customizable test and other features will be added
