import React, {useCallback, useContext, useEffect} from "react";
import AccountBox from "./AccountBox";
import SenderAccounts from "./model/SenderAccounts";
import {BackendSettingsContext} from "./BackendSettingsProvider";
import {backendFetch} from "./common/BackendCall";
import {useConfig} from "./ConfigProvider";
import AccountBalance from "./model/AccountBalance";
import {ethers} from "ethers";

const Accounts = () => {
    const [accounts, setAccounts] = React.useState<SenderAccounts | null>(null);
    const {backendSettings} = useContext(BackendSettingsContext);
    const [selectedAccount, setSelectedAccount] = React.useState<string | null>(null);
    const [selectedChain, setSelectedChain] = React.useState<string | null>("17000");
    const [accountBalance, setAccountBalance] = React.useState<AccountBalance | null>(null);
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
        setAccountBalance(response_json)
    }, [selectedAccount, selectedChain]);


    function row(account: string, i: number) {
        return <AccountBox key={i} account={account}/>;
    }

    useEffect(() => {
        loadTxCount().then();
    }, [loadTxCount]);
    useEffect(() => {
        loadBalance().then();
    }, [loadBalance, selectedAccount, selectedChain]);


    const [inputTo, setInputTo] = React.useState<string>("");
    const [inputAmount, setInputAmount] = React.useState<string>("");
    const [inputToValid, setInputToValid] = React.useState<boolean>(false);
    const [inputAmountValid, setInputAmountValid] = React.useState<boolean>(false);
    const [inputAmountBigInt, setInputAmountBigInt] = React.useState<bigint>(BigInt(0));
    const [inputUseGas, setInputUseGas] = React.useState<boolean>(false);

    useEffect(() => {
        setInputToValid(ethers.utils.isAddress(inputTo));
    }, [inputTo]);
    useEffect(() => {
        try {
            const amount = BigInt(inputAmount);
            setInputAmountValid(true);
            setInputAmountBigInt(amount);
        } catch (e) {
            setInputAmountValid(false);
        }
    }, [inputAmount]);


    const sendTransfer = useCallback(async () => {
        if (inputToValid && selectedChain) {

            const response = await backendFetch(backendSettings, `/transfers/new`, {
                method: "POST",
                body: JSON.stringify({
                    "from": selectedAccount,
                    "to": inputTo,
                    "amount": inputAmountBigInt.toString(),
                    "chain": parseInt(selectedChain),
                    "token": inputUseGas ? null : config.chainSetup[parseInt(selectedChain)].glmAddress,
                }),
            })
            const response_json = await response.text();
            console.log(response_json)
        }
    }, [selectedAccount, inputTo, inputToValid, selectedChain, inputAmountBigInt, inputUseGas, config]);

    return (
        <div>
            <h1>Accounts</h1>
            <div style={{padding: 10}}>
                <select onChange={e => setSelectedAccount(e.target.value)}>
                    {accounts?.publicAddr.map((account) => (
                        <option key={account}>{account}</option>
                    ))}
                </select>
                <div>
                    <select onChange={e => setSelectedChain(e.target.value)}>
                        {Object.entries(config.chainSetup).map(([idx, account]) => (
                            <option key={idx} value={idx}
                                    selected={idx == selectedChain}>{idx} - {account.chainName}</option>
                        ))}
                    </select>
                </div>
                <div>
                    Selected account: {selectedAccount}
                    <div>
                        Gas balance: {accountBalance?.gasBalance}
                    </div>
                    <div>
                        Token balance: {accountBalance?.tokenBalance}
                    </div>

                    Create transfer:

                    <div style={{display: "flex", flexDirection: "column", padding: 20}}>
                        <h4>
                            Create transfer
                        </h4>
                        <input type="text" placeholder="To (address)" onChange={e => setInputTo(e.target.value)}
                               value={inputTo}/>
                        {inputToValid ? inputTo : "Invalid address"}
                        <input type="text" placeholder="Amount" onChange={e => setInputAmount(e.target.value)}
                               value={inputAmount}/>
                        {inputAmountValid ? inputAmountBigInt.toString() : "Invalid amount"}
                        <select>
                            <option selected={inputUseGas} onSelect={e => setInputUseGas(true)}>Gas</option>
                            <option selected={!inputUseGas} onSelect={e => setInputUseGas(false)}>GLM token
                                ({selectedChain ? config.chainSetup[parseInt(selectedChain)].glmAddress : ""})
                            </option>
                        </select>
                        <button onClick={e => sendTransfer()}>Send</button>
                    </div>


                </div>
            </div>
            <div style={{border: "1px solid green"}}>
                DEBUG
                {accounts?.publicAddr.map(row)}
                {JSON.stringify(accounts)}
                {JSON.stringify(config)}
            </div>
        </div>
    );
};

export default Accounts;
