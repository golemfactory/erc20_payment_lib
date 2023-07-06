interface ChainSetup {
    chainName: string;
    confirmationBlocks: number;
    currencyGasSymbol: string;
    currencyGlmSymbol: string;
    faucetEthAmount: number | null;
    faucetGlmAmount: number | null;
    gasLeftWarningLimit: number;
    glmAddress: string;
    maxFeePerGas: string;
    multiContractAddress: string | null;
    multiContractMaxAtOnce: number | null;
    priorityFee: string;
    skipMultiContractCheck: boolean;
    transactionTimeout: number;
    blockExplorerUrl: string;
}

export default ChainSetup;
