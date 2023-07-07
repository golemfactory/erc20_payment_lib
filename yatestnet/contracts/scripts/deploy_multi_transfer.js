// We require the Hardhat Runtime Environment explicitly here. This is optional
// but useful for running the script in a standalone fashion through `node <script>`.
//
// You can also run a script with `npx hardhat run <script>`. If you do that, Hardhat
// will compile your contracts, add the Hardhat Runtime Environment's members to the
// global scope, and execute the script.
const hre = require("hardhat");
const {ethers} = require("hardhat");
const fs = require('fs').promises;


async function getContractFactoryAbi() {
    const contractName = "ContractFactory"
    let compiledContract = await fs.readFile(`artifacts/contracts/${contractName}.sol/${contractName}.json`);
    const contract = JSON.parse(compiledContract);
    return contract.abi;
}

function prepareHashData(creatorAddress, saltHex, bytecodeHash) {
    return ['ff', creatorAddress, saltHex, bytecodeHash].map(x => x.replace('0x', '')).join('');
}

async function main() {
    let config = hre.network.config;
    if (config.contractFactory == undefined) {
        console.log("Deploy contract factory first and set CONTRACT_FACTORY address in .env file");
        return;
    }
    const signers = await hre.ethers.getSigners();
    const signer = signers[0];
    const provider = signer.provider;
    const pubAddr = signer.address;

    let balance = await provider.getBalance(pubAddr);
    console.log(`Using account ${pubAddr} Account balance: ${balance}`);

    //load MultiTransferERC20.json bytecode, compile contract first if not working
    const contractName = "MultiTransferERC20"
    let compiledContract = await fs.readFile(`artifacts/contracts/${contractName}.sol/${contractName}.json`);
    const contract = JSON.parse(compiledContract);
    const contractBytecode = contract.bytecode;

    //handle constructor params for contract
    let constructorParams = hre.ethers.utils.defaultAbiCoder.encode(['address'], [config.glmToken]).slice(2);
    console.log("Constructor params:", constructorParams);

    const bytecode = `${contractBytecode}${constructorParams}`

    const factoryAddress = config.contractFactory.toLowerCase();

    let saltNum = BigInt(1);
    let templateStr = "TEMPLATETEMPLATETEMPLATETEMPLATETEMPLATETEMPLATETEMPLATETEMPLATE";
    console.assert(templateStr.length == 64, "Template string must be 64 characters long");
    let saltStr = "0x" + templateStr;
    let contractAddr = "";
    let bytecodeHash = ethers.utils.keccak256(bytecode);

    let preparedHashTemplate = prepareHashData(factoryAddress, saltStr, bytecodeHash);
    console.log("Prepared hash template for create2:", preparedHashTemplate);
    if (process.env.USE_PREDEFINED_SALT === "1") {
        console.log("USE_PREDEFINED_SALT is set, using predefined salt");
        saltStr = process.env.PREDEFINED_SALT;
        let data = preparedHashTemplate.replace(templateStr, saltStr);
        contractAddr = "0x" + ethers.utils.keccak256(`0x${data}`).slice(-40);
    }
    else {
        const prefix = config.contractPrefix.toLowerCase();
        console.log("Searching for contract address with prefix: " + prefix);
        while (true) {
            saltStr = saltNum.toString(16).padStart(64, '0');
            let data = preparedHashTemplate.replace(templateStr, saltStr);
            contractAddr = "0x" + ethers.utils.keccak256(`0x${data}`).slice(-40);
            if (contractAddr.startsWith(prefix)){
                console.log(`Computed address: ${contractAddr} with salt: ${saltStr}`);
                break;
            }
            saltNum += BigInt(1);
        }
    }
    if (process.env.FIND_SALT_ONLY === "1") {
        console.log("PREPARE_SALT_ONLY is set, exiting");
        return;
    }

    let deployer = new ethers.Contract(factoryAddress, await getContractFactoryAbi(), signer);
    let tx = await deployer.deploy(bytecode, "0x" + saltStr);

    console.log(`Contract deployed to address: ${contractAddr}. Tx id: ${tx.hash}`);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
});
