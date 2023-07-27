import random
import secrets
import string
from eth_account import Account


accounts = []
# iterate
for i in range(0, 41):
    private_key = secrets.token_hex(32)
    public_key = Account.from_key(private_key).address
    eth_value = 2 ** i * 1000 * 1000000000

    accounts.append({
        "private_key": private_key,
        "public_key": public_key,
        "eth_value": eth_value,
    })
for account in accounts:
    print("{},{},{},{}".format(account["private_key"], account["public_key"], account["eth_value"],account["eth_value"] / 1000000000 / 1000000000))
for account in accounts:
    print('"{}": {}"balance": "{}"{}'.format(account["public_key"], '{', account["eth_value"], "},"))

for i in range(0, 10):
    private_key = secrets.token_hex(32)
    public_key = Account.from_key(private_key).address
    eth_value = 1000000000
    print("{},{},{},{}".format(i, private_key, public_key, eth_value))
