require('dotenv').config({path: '.env'})

require("@nomicfoundation/hardhat-toolbox");

private_key = process.env.MAIN_ACCOUNT_PRIVATE_KEY || "0x0000000000000000000000000000000000000000000000000000000000000000";

/** @type import('hardhat/config').HardhatUserConfig */
module.exports = {
    defaultNetwork: "dev",
    networks: {
        dev: {
            url: process.env.YATESTNET_RPC || "http://127.0.0.1:8545",
            accounts: [private_key],
            chainId: 987789
        },
        mumbai: {
            url: process.env.MUMBAI_RPC || "http://127.0.0.1:8545",
            accounts: [private_key],
            chainId: 80001
        }
    },
    solidity: {
        version: "0.8.17",
        settings: {
            optimizer: {
                enabled: true,
                runs: 200
            }
        }
    },
    paths: {
        sources: "./contracts",
        tests: "./test",
        cache: "./cache",
        artifacts: "./artifacts"
    },
    mocha: {
        timeout: 40000
    }
};
