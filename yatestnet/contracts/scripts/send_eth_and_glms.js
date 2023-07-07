const hre = require("hardhat");
const {promises: fs} = require("fs");
const {ethers} = require("hardhat");



async function sendEthAndGlms() {
  const signers = await hre.ethers.getSigners();
  const signer = signers[0];
  const provider = signer.provider;
  const pubAddr = signer.address;

  const targetAddr = process.env.ETH_GLM_SEND_TARGET;
  let amountEth = parseFloat(process.env.ETH_SEND_AMOUNT);
  let amountGlm = parseFloat(process.env.GLM_SEND_AMOUNT);

  amountEth = Math.round(amountEth * 1000000)
  amountGlm = Math.round(amountGlm * 1000000)

  let balance = await provider.getBalance(pubAddr);
  console.log(`Using account ${pubAddr} Account balance: ${balance}`);

  if (balance.eq(0)) {
    console.log("Account balance is 0. Exiting.");
    return;
  }
  const erc20Factory = await hre.ethers.getContractFactory("ERC20");
  const BIG_12 = hre.ethers.BigNumber.from("1000000000000");
  const erc20Contract = await erc20Factory.attach(process.env.GLM_CONTRACT_ADDRESS);
  await erc20Contract.transfer(targetAddr, BIG_12.mul(amountGlm));
  console.log(`Transferred ${amountGlm/1000000} GLM tokens to ${targetAddr}`);

  let tx_sendEther = {
    from: process.env.MAIN_ACCOUNT_PUBLIC_ADDRESS,
    to: targetAddr,
    // Convert currency unit from ether to wei
    value: BIG_12.mul(amountEth)
  }
  let wallet = new ethers.Wallet(process.env.MAIN_ACCOUNT_PRIVATE_KEY, provider)
  await wallet.sendTransaction(tx_sendEther);
  console.log(`Transferred ${amountEth/1000000} ethers to ${targetAddr}`);

  /*
  let glm_token = erc20Contract.address;
  console.log("GLM ERC20 test token deployed to:", glm_token);

  const cf = await hre.ethers.getContractFactory("MultiTransferERC20");

  const multiTransfer = await cf.deploy(glm_token);
  await multiTransfer.deployed();
  console.log("MultiTransferERC20 deployed to:", multiTransfer.address);

  const distr = await hre.ethers.getContractFactory("Distribute");

  const faucetDistr = await distr.deploy(glm_token);
  await faucetDistr.deployed();
  console.log("Faucet deployed to:", faucetDistr.address);


  const res = await erc20Contract.approve(multiTransfer.address, BIG_18.mul(1000000000));
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
    amounts.push(BIG_18.mul(100000000));
  }

  let tx = await multiTransfer.golemTransferDirect(addr_list, amounts);
  let tx_receipt = await tx.wait();*/
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
sendEthAndGlms().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
