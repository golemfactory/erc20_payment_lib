import json
import os
import asyncio
import secrets
import subprocess
import sys
import threading
from eth_account import Account
from dotenv import load_dotenv


def gen_key_address_pair():
    private_key = "0x" + secrets.token_hex(32)
    account_1 = Account.from_key(private_key).address
    return account_1, private_key


def capture_output(process):
    while True:
        output = process.stdout.readline()
        if output:
            print(output.strip())
        else:
            break

    process.communicate()


async def main():
    load_dotenv()
    chain_num = 987789
    data_dir = 'genesis/chaindata'
    chain_dir = f"{data_dir}/chain{chain_num}"
    genesis_file = f"{data_dir}/genesis{chain_num}.json"
    signer_password_file = f"{data_dir}/password{chain_num}.json"
    geth_command_file = f"{data_dir}/geth-command{chain_num}.sh"

    # get private key from env
    main_account = os.environ['MAIN_ACCOUNT_PRIVATE_KEY']
    faucet_account = os.environ['FAUCET_ACCOUNT_PRIVATE_KEY']
    signer_account = os.environ['SIGNER_ACCOUNT_PRIVATE_KEY']
    keystore_password = os.environ['SIGNER_ACCOUNT_KEYSTORE_PASSWORD']
    try:
        period = int(os.environ['PERIOD_IN_SECONDS_INT'])
    except Exception as ex:
        print(f"PERIOD_IN_SECONDS_INT is not set or not int: {ex}")
        period = 5
    keep_running = int(os.environ['KEEP_RUNNING']) == 1

    (main_address, main_account_private_key) = (
        Account.from_key(main_account).address,
        main_account)
    (faucet_address, faucet_account_private_key) = (
        Account.from_key(faucet_account).address,
        faucet_account)

    print(f"Loaded main account: {main_address}")

    (signer_address, signer_private_key) = (
        Account.from_key(signer_account).address,
        signer_account)

    deploy_contracts = False
    if not os.path.exists(data_dir):
        deploy_contracts = True

        os.makedirs(data_dir)

        print(f"Loaded signer account: {signer_address}")

        genesis = {
            "config": {
                "chainId": chain_num,
                "homesteadBlock": 0,
                "eip150Block": 0,
                "eip155Block": 0,
                "eip158Block": 0,
                "byzantiumBlock": 0,
                "constantinopleBlock": 0,
                "petersburgBlock": 0,
                "istanbulBlock": 0,
                "berlinBlock": 0,
                "londonBlock": 0,
                "ArrowGlacierBlock": 0,
                "GrayGlacierBlock": 0,
                "clique": {
                    "period": period,
                    "epoch": 0
                }
            },
            "difficulty": "1",
            "gasLimit": "30000000",
            # Signer address for clique
            "extradata": "0x0000000000000000000000000000000000000000000000000000000000000000"
                         + f"{signer_address}".lower().replace("0x", "")
                         + "000000000000000000000000000000000000000000000000000000000000000000"
                         + "0000000000000000000000000000000000000000000000000000000000000000",
            "alloc": {
                main_address: {"balance": '1000000000000000000000000000'},
                "0xB1C4D937A1b9bfC17a2Eb92D3577F8b66763bfC1": {"balance": "1000000000000"},
                "0x4799b810050f038288b4314501b70B1B9A49E1Dc": {"balance": "2000000000000"},
                "0xAc630277FB747Aa600d7A23EF08F5829861c639E": {"balance": "4000000000000"},
                "0xc48878a43476cd6cC5db772c492cB68D6d201249": {"balance": "8000000000000"},
                "0x0C5bE0eF7Fab4E847DD7bcc642a203220C730f21": {"balance": "16000000000000"},
                "0x1e97A59959394A7f3DFa753d1b8B12100b5d7Ce8": {"balance": "32000000000000"},
                "0x7754e3AE9A42D1Ad76afD691f1cFc7f0D4a82698": {"balance": "64000000000000"},
                "0x4caa30c14bC74bF3099CBe589a37DE53A4855EF6": {"balance": "128000000000000"},
                "0xEFac7290De2728630a4819C8443b4236a45B3e21": {"balance": "256000000000000"},
                "0x5774B9c27fAe1339386dED640fdc2717bCeD07C9": {"balance": "512000000000000"},
                "0x4E6076728Ba724Fc327B115ad3CEDB8aCbe37bd8": {"balance": "1024000000000000"},
                "0x32Fc1A423F2B4aC21bD2679bD160e418598ACFC7": {"balance": "2048000000000000"},
                "0xb33266F2A44209Fdb59bdc98feB6474DB1cF83E0": {"balance": "4096000000000000"},
                "0x7FEDa0B256EB12FCFEec66f44F9e12CC631F0Df9": {"balance": "8192000000000000"},
                "0xf77358be76125E0f95e206E24F1036C9F49D9692": {"balance": "16384000000000000"},
                "0xff68350f138C4eB632beE2B59F640ab6d1e2e475": {"balance": "32768000000000000"},
                "0xA9014205808373CeF5b6815f50e03842a99a9206": {"balance": "65536000000000000"},
                "0x368E33F48F52755221B97389327B2eFf97c32700": {"balance": "131072000000000000"},
                "0xa7ba45b534526513C0405e562cbbCDA50872a851": {"balance": "262144000000000000"},
                "0x7bd3674a3212652D685488b6401Ef61452bEBB79": {"balance": "524288000000000000"},
                "0xe4458E5080d9D8f39c235cc8B2090cDB02881925": {"balance": "1048576000000000000"},
                "0x4e94C42d9b7cBD4c8ae8254d0Cb2884e0a2055ac": {"balance": "2097152000000000000"},
                "0xEFa492B64cca91686Ed2FBbea29783C7b834CDDA": {"balance": "4194304000000000000"},
                "0x676e15C9375a925fbc1b0891f555D884788575cE": {"balance": "8388608000000000000"},
                "0xE6F185DAe234bC4369cFF548556A6E1Ce34A07E9": {"balance": "16777216000000000000"},
                "0xb9516A91e2a5F696430EEdc78d4F911f284DF35e": {"balance": "33554432000000000000"},
                "0x42a3906dEf13106ADCe76dC93405b354da3e2035": {"balance": "67108864000000000000"},
                "0xd4052DAbC05e0A4B04F493612af2e5D1055978ac": {"balance": "134217728000000000000"},
                "0x1eA5eeAD1Ba9CCD7A026f226c5e48e8781573562": {"balance": "268435456000000000000"},
                "0xbfb29b133aA51c4b45b49468F9a22958EAFeA6fa": {"balance": "536870912000000000000"},
                "0x653b48E1348F480149047AA3a58536eb0dbBB2E2": {"balance": "1073741824000000000000"},
                "0x2E9e88A1f32Ea12bBaF3d3eb52a71c8224451431": {"balance": "2147483648000000000000"},
                "0x40982A8F07A39DA509581751648efCadB276f4E9": {"balance": "4294967296000000000000"},
                "0x9Ad40e3D756F59949485A280c572d8e715F14350": {"balance": "8589934592000000000000"},
                "0x805D24c97d6dDFa63F402b8A5e16491229523a96": {"balance": "17179869184000000000000"},
                "0x0E7E1c5aF8e3EA87527242a12C7A30e7E686090D": {"balance": "34359738368000000000000"},
                "0x53fB152b2f69a48Bf1387f742e254725E5dB6b23": {"balance": "68719476736000000000000"},
                "0x352734dAff396a59B56366b0A3C2A642B7643267": {"balance": "137438953472000000000000"},
                "0x7372CAe62B3E5014dCC1060bA3741DeDBa28C7BB": {"balance": "274877906944000000000000"},
                "0x6ae57Ecaeb101cc9CC0b9575CEC084B5cd39a8c6": {"balance": "549755813888000000000000"},
                "0x001DA7D21181D3a3Bc8D88A2faCDB6AE7DFB10E8": {"balance": "1099511627776000000000000"},
                "0x55300627b2714D87649c31d2983e40301F0Cac89": {"balance": '1000000000000000000000000000'}
            }
        }

        with open(f'{genesis_file}', 'w') as f:
            json.dump(genesis, f, indent=4)

        os.system(f'geth --datadir {chain_dir} init {genesis_file}')

        keystore = Account.encrypt(signer_account, keystore_password)

        with open(f'{chain_dir}/keystore/signer_key', 'w') as f:
            f.write(json.dumps(keystore, indent=4))
        with open(signer_password_file, 'w') as f:
            f.write(keystore_password)

    # clique signer/miner settings
    geth_command = f'geth --datadir={chain_dir} ' \
                   f'--nodiscover ' \
                   f'--syncmode=full ' \
                   f'--gcmode=archive ' \
                   f'--http ' \
                   f'--http.addr=0.0.0.0 ' \
                   f'--http.vhosts=* ' \
                   f'--http.corsdomain=* ' \
                   f'--http.api=eth,net,web3,txpool,debug ' \
                   f'--rpc.txfeecap=1000 ' \
                   f'--networkid={chain_num}'
    enable_mining = True
    if enable_mining:
        miner_settings = f" --mine --miner.etherbase={signer_address} --allow-insecure-unlock --unlock={signer_address} --password={signer_password_file}"
        geth_command += miner_settings
    enable_metrics = True
    if enable_metrics:
        metrics_settings = f" --metrics --metrics.addr=0.0.0.0 --metrics.expensive"
        geth_command += metrics_settings
    disable_ipc = True
    if disable_ipc:
        geth_command += " --ipcdisable"
    print(geth_command)
    with open(geth_command_file, "w") as f:
        f.write(geth_command)

    geth_command_split = geth_command.split(' ')
    process = subprocess.Popen(geth_command_split, stdout=subprocess.PIPE)
    thread = threading.Thread(target=capture_output, args=(process,))
    thread.start()
    # give the process some time to start http server
    await asyncio.sleep(2)

    if deploy_contracts:
        # deploy contracts
        os.chdir("contracts")
        os.system("npm run deploy_dev")

    print("Blockchain is ready for testing")

    if not keep_running:
        print("Testing complete. Shutdown blockchain")
        process.kill()
        thread.join()
        print(geth_command)
    else:
        while True:
            await asyncio.sleep(2)


if __name__ == "__main__":
    if sys.platform == 'win32':
        # Set the policy to prevent "Event loop is closed" error on Windows - https://github.com/encode/httpx/issues/914
        asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

    asyncio.run(main())
