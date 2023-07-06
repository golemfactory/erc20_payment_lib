interface TokenTransfer {
    id: number;
    chainId: number | string;
    txId: number;
    fromAddress: string;
    receiverAddr: string;
    tokenAddr: string | null;
    tokenAmount: string;
}

export default TokenTransfer;
