set -x
export RUST_LOG=info,erc20_rpc_pool=error
# generate random int
depositId=$(shuf -i 0-2000000000 -n 1)
echo "Deposit ID: $depositId"
./target_wsl/debug/erc20_processor generate-key -n 3 > .env
cat .env | grep ETH_ADDRESS | sed "s/#\s/export /g" | sed "s/:\s/=/g" > load_env.sh
source load_env.sh
echo "Address 0: $ETH_ADDRESS_0"
echo "Address 1: $ETH_ADDRESS_1"
echo "Address 2: $ETH_ADDRESS_2"


./target_wsl/debug/erc20_processor get-dev-eth --address $ETH_ADDRESS_1
./target_wsl/debug/erc20_processor get-dev-eth --address $ETH_ADDRESS_2
sleep 30
./target_wsl/debug/erc20_processor mint-test-tokens --address $ETH_ADDRESS_1
./target_wsl/debug/erc20_processor mint-test-tokens --address $ETH_ADDRESS_2
./target_wsl/debug/erc20_processor run
sleep 20
./target_wsl/debug/erc20_processor balance
./target_wsl/debug/erc20_processor deposit --account-no 1 --amount 800
./target_wsl/debug/erc20_processor run
sleep 20
./target_wsl/debug/erc20_processor make-deposit --account-no 1 --amount 500 --fee-amount 100 --block-for 1000 --spender $ETH_ADDRESS_2 --deposit-id $depositId --use-internal
./target_wsl/debug/erc20_processor run
sleep 20
./target_wsl/debug/erc20_processor check-deposit --deposit-id $depositId
./target_wsl/debug/erc20_processor transfer --deposit-id $depositId --account-no 2 --amount 0.0001 --recipient $ETH_ADDRESS_0 --use-internal
./target_wsl/debug/erc20_processor run
sleep 20
./target_wsl/debug/erc20_processor balance
./target_wsl/debug/erc20_processor transfer --deposit-id $depositId --account-no 2 --amount 0.0001 --recipient 0x0000000000000000000000000000000000000001 --use-internal
./target_wsl/debug/erc20_processor transfer --deposit-id $depositId --account-no 2 --amount 0.0001 --recipient 0x0000000000000000000000000000000000000002 --use-internal
./target_wsl/debug/erc20_processor transfer --deposit-id $depositId --account-no 2 --amount 0.0001 --recipient 0x0000000000000000000000000000000000000003 --use-internal
./target_wsl/debug/erc20_processor transfer --deposit-id $depositId --account-no 2 --amount 0.0001 --recipient 0x0000000000000000000000000000000000000004 --use-internal
./target_wsl/debug/erc20_processor run
sleep 20
./target_wsl/debug/erc20_processor balance