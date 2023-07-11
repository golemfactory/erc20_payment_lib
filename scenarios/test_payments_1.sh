# Scenario: Test payments
export RUST_LOG=error
erc20_processor generate-test-payments -a -c dev --generate-count 1000 --address-pool-size 1000 --amounts-pool-size 100000 &
unset RUST_LOG
erc20_processor run --keep-running
