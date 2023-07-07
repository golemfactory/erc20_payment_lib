# Script for create .env file with random accounts
# Look at README.md for more details

import random
import secrets
import string
from eth_account import Account

private_key = "0x" + secrets.token_hex(32)
print(f"# Signer account used for block producing", )
print(f"SIGNER_ACCOUNT_PRIVATE_KEY={private_key}", )
print(f"SIGNER_ACCOUNT_PUBLIC_ADDRESS={Account.from_key(private_key).address}")
random_pass = ''.join(random.choice(string.ascii_lowercase + string.ascii_uppercase + string.digits) for _ in range(20))
print(f"SIGNER_ACCOUNT_KEYSTORE_PASSWORD={random_pass}")

print("")
print("# Main account (like administrator account)")
private_key = "0x" + secrets.token_hex(32)
print(f"MAIN_ACCOUNT_PRIVATE_KEY={private_key}")
print(f"MAIN_ACCOUNT_PUBLIC_ADDRESS={Account.from_key(private_key).address}")

print("")
print("# Main faucet")
private_key = "0x" + secrets.token_hex(32)
print(f"FAUCET_ACCOUNT_PRIVATE_KEY={private_key}")
print(f"FAUCET_ACCOUNT_PUBLIC_ADDRESS={Account.from_key(private_key).address}")

print("# Fill these values after deploying contracts")
print(f"GLM_CONTRACT_ADDRESS=fill_me")
print(f"MULTI_PAYMENT_CONTRACT_ADDRESS=fill_me")
print(f"DISTRIBUTE_CONTRACT_ADDRESS=fill_me")

