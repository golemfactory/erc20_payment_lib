interface ChainTransfer {
    id: number;
    chainId: number;
    chainTxId: number;
    fromAddress: string;
    receiverAddress: string;
    tokenAddress: string;
    tokenAmount: string;
    blockchainDate: string;
}

export default ChainTransfer;
