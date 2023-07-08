# Scenario: Test payments
erc20_processor generate-test-payments -a -c dev --interval 0.4 --generate-count 1000000000 --address-pool-size 1000 --amounts-pool-size 100000 &
erc20_processor run --keep-running
