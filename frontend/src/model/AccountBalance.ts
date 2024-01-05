interface AccountBalance {
    networkId: number;
    account: string;
    gasBalance: string;
    tokenBalance: string;
    depositBalance: string | null;
    blockNumber: number;
    blockDate: string;
}

export default AccountBalance;
