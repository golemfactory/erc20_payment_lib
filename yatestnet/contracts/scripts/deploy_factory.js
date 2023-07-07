// We require the Hardhat Runtime Environment explicitly here. This is optional
// but useful for running the script in a standalone fashion through `node <script>`.
//
// You can also run a script with `npx hardhat run <script>`. If you do that, Hardhat
// will compile your contracts, add the Hardhat Runtime Environment's members to the
// global scope, and execute the script.
const hre = require("hardhat");

async function main() {
    const signers = await hre.ethers.getSigners();
    const signer = signers[0];
    const provider = signer.provider;
    const pubAddr = signer.address;

    let balance = await provider.getBalance(pubAddr);
    console.log(`Using account ${pubAddr} Account balance: ${balance}`);

    if (balance.eq(0)) {
        console.log("Account balance is 0. Exiting.");
        return;
    }
    const erc20Factory = await hre.ethers.getContractFactory("contracts/ERC20Contract.sol:ERC20");
    const BIG_18 = hre.ethers.BigNumber.from("1000000000000000000");
    const erc20Contract = await erc20Factory.deploy(pubAddr, BIG_18.mul(1000000000000));
    await erc20Contract.deployed();
    let glm_token = erc20Contract.address;
    console.log("GLM ERC20 test token deployed to:", glm_token);

    const cf = await hre.ethers.getContractFactory("MultiTransferERC20");

    const multiTransfer = await cf.deploy(glm_token);
    await multiTransfer.deployed();
    console.log("MultiTransferERC20 deployed to:", multiTransfer.address);

    const distr = await hre.ethers.getContractFactory("Distribute");

    const faucetDistr = await distr.deploy(glm_token);
    await faucetDistr.deployed();
    console.log("Distribute contract deployed to:", faucetDistr.address);


    const res = await erc20Contract.approve(multiTransfer.address, BIG_18.mul(1000000000000));
    const receipt = await res.wait();
    console.log("Approve result: ", receipt.status);

    faucet_addr = process.env.FAUCET_ACCOUNT_PUBLIC_ADDRESS;
    let tx_sendEther = {
        from: process.env.MAIN_ACCOUNT_PUBLIC_ADDRESS,
        to: faucet_addr,
        // Convert currency unit from ether to wei
        value: BIG_18.mul(100000000)
    }
    let wallet = new ethers.Wallet(process.env.MAIN_ACCOUNT_PRIVATE_KEY, provider)
    await wallet.sendTransaction(tx_sendEther);

    let addr_list = [faucet_addr];
    let amounts = [];
    for (let addr in addr_list) {
        amounts.push(BIG_18.mul(100000000000));
    }

    let tx = await multiTransfer.golemTransferDirect(addr_list, amounts);
    let tx_receipt = await tx.wait();
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
});
