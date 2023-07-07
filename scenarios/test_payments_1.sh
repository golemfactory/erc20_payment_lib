# Scenario: Test payments
generate_transfers --chain-name dev --generate-count 100 --address-pool-size 100 --amounts-pool-size 10000
erc20_processor run
