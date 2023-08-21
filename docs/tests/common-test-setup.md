## Common test setup

### Database

By default, in-memory is used for tests, you can change that by adding environment variable:

```
ERC20_TESTS_USE_DISK_DB=1  
```

You can force test file name, but it can lead to failed tests due to multiple tests using the same database, or opening old database. It should work when running one test at once and cleaning db before run.

```
ERC20_TESTS_OVERRIDE_DB_NAME=erc20lib.sqlite  
```

### Docker

Docker is spawned at the start of the test and removed. 

If you want docker to keep running after end of the test you can set  
```
ERC20_TEST_KEEP_DOCKER_CONTAINER=1
```

For each test docker has specified timeout after it is closed and removed, but setting this variable (ERC20_TEST_KEEP_DOCKER_CONTAINER) should allow docker to be run as long as you want.



