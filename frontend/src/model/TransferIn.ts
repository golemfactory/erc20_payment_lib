interface TransferIn {
    id: number;
    chainId: number;
    blockchainDate: string;
    chainTxId: number;
    fromAddress: string;
    receiverAddress: string;
    requestedDate: string;
    tokenAddress: string;
    tokenAmount: string;
    txHash: string | null;
}

export default TransferIn;
