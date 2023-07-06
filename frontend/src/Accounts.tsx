import React, { useCallback, useContext, useEffect } from "react";
import AccountBox from "./AccountBox";
import SenderAccounts from "./model/SenderAccounts";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { backendFetch } from "./common/BackendCall";

const Accounts = () => {
    const [accounts, setAccounts] = React.useState<SenderAccounts | null>(null);
    const { backendSettings } = useContext(BackendSettingsContext);

    const loadTxCount = useCallback(async () => {
        const response = await backendFetch(backendSettings, "/accounts");
        const response_json = await response.json();
        setAccounts(response_json);
    }, []);

    function row(account: string, i: number) {
        return <AccountBox key={i} account={account} />;
    }

    useEffect(() => {
        loadTxCount().then();
    }, [loadTxCount]);
    return (
        <div>
            <h1>Accounts</h1>
            {accounts?.publicAddr.map(row)}
            {JSON.stringify(accounts)}
        </div>
    );
};

export default Accounts;
