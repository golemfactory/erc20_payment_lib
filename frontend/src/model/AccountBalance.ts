interface AccountBalance {
    networkId: number;
    account: string;
    gasBalance: string;
    tokenBalance: string;
    depositBalance: string | null;
}

export default AccountBalance;