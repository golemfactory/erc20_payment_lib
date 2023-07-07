import argparse
import os

# This wrapper script is needed because somehow hardhat don't like to get arguments from command line
# So the environment variables are used instead

parser = argparse.ArgumentParser(
    prog='send_eth_and_glms.py',
    description='Send ETH and GLMs to an address')

parser.add_argument('--address', help='Address to send ETH and GLMs to', required=True)
parser.add_argument('--eth', default="1.0")
parser.add_argument('--glm', default="1.0")

args = parser.parse_args()
os.environ["ETH_GLM_SEND_TARGET"] = args.address
os.environ["ETH_SEND_AMOUNT"] = args.eth
os.environ["GLM_SEND_AMOUNT"] = args.glm

npx_command_split = "npx hardhat run --network dev scripts/send_eth_and_glms.js"
os.chdir("contracts")
os.system(npx_command_split)
