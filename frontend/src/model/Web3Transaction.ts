interface Web3Transaction {
    id: number;
    feePaid: string;
    confirmDate: string | null;
    error: string | null;
    engineError: string | null;
    engineMessage: string | null;
    chainId: number;
    method: string;
    createdDate: string;
    fromAddr: string;
    toAddr: string;
    broadcastDate: string | null;
    txHash: string | null;
    nonce: number | null;
    gasLimit: number | null;
}

export default Web3Transaction;
