import React, { useCallback, useContext, useEffect } from "react";
import AccountBox from "./AccountBox";
import SenderAccounts from "./model/SenderAccounts";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { backendFetch } from "./common/BackendCall";
import { useConfig } from "./ConfigProvider";
import CreateTransferBox from "./CreateTransferBox";
import CurrentBalanceBox from "./CurrentBalanceBox";
import EventBox from "./EventBox";
import Web3Box from "./Web3Box";

const Accounts = () => {
    const [accounts, setAccounts] = React.useState<SenderAccounts | null>(null);
    const { backendSettings } = useContext(BackendSettingsContext);
    const [selectedAccount, setSelectedAccount] = React.useState<string | null>(null);
    const [selectedChain, setSelectedChain] = React.useState<string | null>("560048");
    const config = useConfig();

    const loadTxCount = useCallback(async () => {
        const response = await backendFetch(backendSettings, "/accounts");
        const response_json = await response.json();
        setAccounts(response_json);
        setSelectedAccount(response_json.publicAddr[0]);
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
            <div>
                <EventBox selectedChain={null}></EventBox>
            </div>
            <div>
                <Web3Box selectedChain={selectedChain}></Web3Box>
            </div>

            <div style={{ padding: 10 }}>
                <select onChange={(e) => setSelectedAccount(e.target.value)}>
                    {accounts?.publicAddr.map((account) => (
                        <option key={account}>{account}</option>
                    ))}
                </select>
                <div>
                    <select onChange={(e) => setSelectedChain(e.target.value)}>
                        {Object.entries(config.chainSetup).map(([idx, account]) => (
                            <option key={idx} value={idx} selected={idx == selectedChain}>
                                {idx} - {account.chainName}
                            </option>
                        ))}
                    </select>
                </div>
                <div>
                    Selected account: {selectedAccount}
                    <CurrentBalanceBox selectedChain={selectedChain} selectedAccount={selectedAccount} />
                    Create transfer:
                    <CreateTransferBox selectedAccount={selectedAccount} selectedChain={selectedChain} />
                </div>
            </div>
            <div style={{ border: "1px solid green" }}>
                DEBUG
                {accounts?.publicAddr.map(row)}
                {JSON.stringify(accounts)}
                {JSON.stringify(config)}
            </div>
        </div>
    );
};

export default Accounts;
