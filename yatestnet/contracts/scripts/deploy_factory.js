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

    const donothing = await hre.ethers.getContractFactory("DoNothingContract");

    const donothingDeployed = await donothing.deploy();
    await donothingDeployed.deployed();
    console.log("Do nothing contract deployed to:", donothingDeployed.address);

    const res = await erc20Contract.approve(multiTransfer.address, BIG_18.mul(1000000000000));
    const receipt = await res.wait();
    console.log("Approve result: ", receipt.status);

    let wallet = new ethers.Wallet(process.env.MAIN_ACCOUNT_PRIVATE_KEY, provider)

    faucet_addr = process.env.FAUCET_ACCOUNT_PUBLIC_ADDRESS;
    let tx_sendEther = {
        from: process.env.MAIN_ACCOUNT_PUBLIC_ADDRESS,
        to: faucet_addr,
        // Convert currency unit from ether to wei
        value: BIG_18.mul(100000000)
    }
    await wallet.sendTransaction(tx_sendEther);

    {
        let tx_sendEther = {
            from: process.env.MAIN_ACCOUNT_PUBLIC_ADDRESS,
            to: "0x8080Dd8cb6CBEc227d26EA402Cc97a482250Ee72",
            // Convert currency unit from ether to wei
            value: BIG_18.mul(100000000)
        }

        await wallet.sendTransaction(tx_sendEther);
    }

    let addr_list = [faucet_addr, "0x8080Dd8cb6CBEc227d26EA402Cc97a482250Ee72",
       "0xB1C4D937A1b9bfC17a2Eb92D3577F8b66763bfC1",
       "0x4799b810050f038288b4314501b70B1B9A49E1Dc",
       "0xAc630277FB747Aa600d7A23EF08F5829861c639E",
       "0xc48878a43476cd6cC5db772c492cB68D6d201249",
       "0x0C5bE0eF7Fab4E847DD7bcc642a203220C730f21",
       "0x1e97A59959394A7f3DFa753d1b8B12100b5d7Ce8",
       "0x7754e3AE9A42D1Ad76afD691f1cFc7f0D4a82698",
       "0x4caa30c14bC74bF3099CBe589a37DE53A4855EF6",
       "0xEFac7290De2728630a4819C8443b4236a45B3e21",
       "0x5774B9c27fAe1339386dED640fdc2717bCeD07C9",
       "0x4E6076728Ba724Fc327B115ad3CEDB8aCbe37bd8",
       "0x32Fc1A423F2B4aC21bD2679bD160e418598ACFC7",
       "0xb33266F2A44209Fdb59bdc98feB6474DB1cF83E0",
       "0x7FEDa0B256EB12FCFEec66f44F9e12CC631F0Df9",
       "0xf77358be76125E0f95e206E24F1036C9F49D9692",
       "0xff68350f138C4eB632beE2B59F640ab6d1e2e475",
       "0xA9014205808373CeF5b6815f50e03842a99a9206",
       "0x368E33F48F52755221B97389327B2eFf97c32700",
       "0xa7ba45b534526513C0405e562cbbCDA50872a851",
       "0x7bd3674a3212652D685488b6401Ef61452bEBB79",
       "0xe4458E5080d9D8f39c235cc8B2090cDB02881925",
       "0x4e94C42d9b7cBD4c8ae8254d0Cb2884e0a2055ac",
       "0xEFa492B64cca91686Ed2FBbea29783C7b834CDDA",
       "0x676e15C9375a925fbc1b0891f555D884788575cE",
       "0xE6F185DAe234bC4369cFF548556A6E1Ce34A07E9",
       "0xb9516A91e2a5F696430EEdc78d4F911f284DF35e",
       "0x42a3906dEf13106ADCe76dC93405b354da3e2035",
       "0xd4052DAbC05e0A4B04F493612af2e5D1055978ac",
       "0x1eA5eeAD1Ba9CCD7A026f226c5e48e8781573562",
       "0xbfb29b133aA51c4b45b49468F9a22958EAFeA6fa",
       "0x653b48E1348F480149047AA3a58536eb0dbBB2E2",
       "0x2E9e88A1f32Ea12bBaF3d3eb52a71c8224451431",
       "0x40982A8F07A39DA509581751648efCadB276f4E9",
       "0x9Ad40e3D756F59949485A280c572d8e715F14350",
       "0x805D24c97d6dDFa63F402b8A5e16491229523a96",
       "0x0E7E1c5aF8e3EA87527242a12C7A30e7E686090D",
       "0x53fB152b2f69a48Bf1387f742e254725E5dB6b23",
       "0x352734dAff396a59B56366b0A3C2A642B7643267",
       "0x7372CAe62B3E5014dCC1060bA3741DeDBa28C7BB",
       "0x6ae57Ecaeb101cc9CC0b9575CEC084B5cd39a8c6",
       "0x001DA7D21181D3a3Bc8D88A2faCDB6AE7DFB10E8",
    ];
    let amounts = [];
    for (let addr in addr_list) {
        amounts.push(BIG_18.mul(1000));
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
