import asyncio
import logging
import argparse
import platform

import batch_rpc_provider
from batch_rpc_provider import BatchRpcProvider, BatchRpcException, check_address_availability, binary_history_check

logging.basicConfig()
logger = logging.getLogger(__name__)
logger.setLevel(logging.DEBUG)

NULL_ADDR = "0x0000000000000000000000000000000000000000"
POLYGON_GENESIS_ADDR = "0x0000000000000000000000000000000000001010"

CHAIN_ID_MAINNET = 1
CHAIN_ID_RINKEBY = 4
CHAIN_ID_GOERLI = 5
CHAIN_ID_POLYGON = 137
CHAIN_ID_MUMBAI = 80001

POLYGON_USD_TOKEN = "0xc2132D05D31c914a87C6611C10748AEb04B58e8F"
CHECK_USD_HOLDER = "0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245"


async def check_history_availability(p: BatchRpcProvider):
    chain_id = await p.get_chain_id()

    def get_addr_to_check():
        if chain_id == CHAIN_ID_MAINNET:
            return NULL_ADDR
        if chain_id == CHAIN_ID_RINKEBY:
            return NULL_ADDR
        if chain_id == CHAIN_ID_GOERLI:
            return NULL_ADDR
        if chain_id == CHAIN_ID_POLYGON:
            return POLYGON_GENESIS_ADDR
        if chain_id == CHAIN_ID_MUMBAI:
            return POLYGON_GENESIS_ADDR
        raise Exception(f"Unrecognized chain id {chain_id}")

    check_balance_addr = get_addr_to_check()

    min_succeeded_block, history_depth = await check_address_availability(p, check_balance_addr)
    # logger.info(f"Seems like history is available from {min_succeeded_block}. History depth: {latest_block - min_succeeded_block}")
    return min_succeeded_block, history_depth


async def check_holder_nozero(p: BatchRpcProvider, token, address):
    latest_block = await p.get_latest_block()

    chain_id = await p.get_chain_id()

    async def check(current_block):
        try:
            logger.info(f"Checking block {current_block}")
            balance = await p.get_erc20_balance(CHECK_USD_HOLDER, POLYGON_USD_TOKEN, f"0x{current_block:x}")
            logger.info(f"Balance at block {current_block} is {balance}")
            if int(balance, 0) <= 0:
                return False
            return True
        except BatchRpcException:
            return False

    min_succeeded_block = await binary_history_check(-1, latest_block, check)
    return min_succeeded_block, latest_block - min_succeeded_block


async def get_holder_history(p: BatchRpcProvider, token, address, min_block, max_block, every_block=1):
    latest_block = await p.get_latest_block()

    chain_id = await p.get_chain_id()

    blocks = []
    block_no = min_block
    while block_no < max_block:
        blocks.append(f"0x{block_no:x}")
        block_no += every_block

    balances = await p.get_erc20_balance_history(address, token, blocks)

    print(balances)



async def get_holder(p: BatchRpcProvider):

    history_block, history_depth = await check_holder_nozero(p, POLYGON_USD_TOKEN, CHECK_USD_HOLDER)
    return history_block, history_depth



async def main():
    parser = argparse.ArgumentParser(description='Test params')
    parser.add_argument('--target-url', dest="target_url", type=str, help='Node name', default="https://polygon-rpc.com")
    parser.add_argument('--action', dest="action", type=str, help='Which action to perform', default="check_history_availability")


    args = parser.parse_args()

    p = BatchRpcProvider(args.target_url, 100)
    if args.action == "check_history_availability":
        res = await check_history_availability(p)
        print("Oldest block: {}, archive depth: {}".format(res[0], res[1]))
    elif args.action == "holder_check":
        res = await get_holder(p)
        print("Oldest block: {}, archive depth: {}".format(res[0], res[1]))
    elif args.action == "holder_history":
        res = await get_holder_history(p, POLYGON_USD_TOKEN, CHECK_USD_HOLDER, 30000000, 34000000, 43200)
    else:
        raise Exception("Unknown action")


if __name__ == "__main__":
    print(batch_rpc_provider.__version__)
    if platform.system() == 'Windows':
        asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

    asyncio.run(main())
