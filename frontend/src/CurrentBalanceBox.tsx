import React, { useCallback, useContext, useEffect } from "react";
import "./CurrentBalanceBox.css";
import { backendFetch } from "./common/BackendCall";
import AccountBalance from "./model/AccountBalance";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import DateBox from "./DateBox";
import { ethers } from "ethers";
import { useConfig } from "./ConfigProvider";

interface CurrentBalanceBoxProps {
    selectedChain: string | null;
    selectedAccount: string | null;
}

const CurrentBalanceBox = (props: CurrentBalanceBoxProps) => {
    const { backendSettings } = useContext(BackendSettingsContext);
    const [accountBalance, setAccountBalance] = React.useState<AccountBalance | null>(null);
    const config = useConfig();
    const loadBalance = useCallback(async () => {
        if (!props.selectedAccount || !props.selectedChain) {
            return;
        }
        const response = await backendFetch(
            backendSettings,
            `/balance/${props.selectedAccount}/${props.selectedChain}`,
        );
        const response_json = await response.json();
        setAccountBalance(response_json);
    }, [backendSettings, props.selectedAccount, props.selectedChain, setAccountBalance]);

    useEffect(() => {
        loadBalance().then();
    }, [loadBalance, props.selectedAccount, props.selectedChain]);

    if (!props.selectedChain || !props.selectedAccount) {
        return <div className={"current-balance-box"}>Select account and chain</div>;
    }

    return (
        <div className={"current-balance-box"}>
            <div className={"current-balance-box-header"}>Current Balance</div>
            <div>
                <DateBox
                    date={accountBalance?.blockDate ?? null}
                    title={"Balance for block number: " + (accountBalance?.blockNumber ?? "")}
                ></DateBox>
            </div>
            <div>
                <div className="current-balance-box-label">Gas balance:</div>
                <div className="current-balance-box-value">
                    {ethers.utils.formatEther(accountBalance?.gasBalance ?? "0")}{" "}
                    {config.chainSetup[parseInt(props.selectedChain)].currencyGasSymbol}
                </div>
            </div>
            <div>
                <div className="current-balance-box-label">Token balance:</div>
                <div className="current-balance-box-value">
                    {ethers.utils.formatEther(accountBalance?.tokenBalance ?? "0")}{" "}
                    {config.chainSetup[parseInt(props.selectedChain)].currencyGlmSymbol}
                </div>
            </div>
        </div>
    );
};

export default CurrentBalanceBox;
