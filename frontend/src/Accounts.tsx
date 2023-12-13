import React, { useCallback, useContext, useEffect } from "react";
import AccountBox from "./AccountBox";
import SenderAccounts from "./model/SenderAccounts";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { backendFetch } from "./common/BackendCall";
import {useConfig} from "./ConfigProvider";

const Accounts = () => {
    const [accounts, setAccounts] = React.useState<SenderAccounts | null>(null);
    const { backendSettings } = useContext(BackendSettingsContext);
    const [selectedAccount, setSelectedAccount] = React.useState<string | null>(null);
    const [selectedChain, setSelectedChain] = React.useState<string | null>(null);
    const config = useConfig();

    const loadTxCount = useCallback(async () => {
        const response = await backendFetch(backendSettings, "/accounts");
        const response_json = await response.json();
        setAccounts(response_json);
        setSelectedAccount(response_json.publicAddr[0])
    }, []);

    const loadBalance = useCallback(async () => {
        if (!selectedAccount || !selectedChain) {
            return
        }
        const response = await backendFetch(backendSettings, `/balance/${selectedAccount}/${selectedChain}`)
        const response_json = await response.json();

    }, [selectedAccount, selectedChain]);


    function row(account: string, i: number) {
        return <AccountBox key={i} account={account} />;
    }

    useEffect(() => {
        loadTxCount().then();
    }, [loadTxCount]);
    useEffect(() => {
        loadBalance().then();
    }, [loadBalance, selectedAccount, selectedChain]);


    return (
        <div>
            <h1>Accounts</h1>
            <div style={{padding:10}}>
                <select onChange={e => setSelectedAccount(e.target.value)}>
                    {accounts?.publicAddr.map((account) => (
                        <option key={account}>{account}</option>
                    ))}
                </select>
                <div>
                    <select onChange={e => setSelectedChain(e.target.value)}>
                        {Object.entries(config.chainSetup).map(([idx, account]) => (
                            <option key={idx} value={idx}>{idx} - {account.chainName}</option>
                        ))}
                    </select>
                </div>
                <div>
                    Selected account: {selectedAccount}
                    <div>
                        Create new transfer
                    </div>
                </div>
            </div>
            <div style={{border:"1px solid green"}}>
                DEBUG
                {accounts?.publicAddr.map(row)}
                {JSON.stringify(accounts)}
                {JSON.stringify(config)}
            </div>
        </div>
    );
};

export default Accounts;
