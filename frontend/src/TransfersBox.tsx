import React, { useContext, useState } from "react";
import "./TransfersBox.css";
import TransferBox from "./TransferBox";
import { useConfig } from "./ConfigProvider";
import TokenTransfer from "./model/TokenTransfer";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { backendFetch } from "./common/BackendCall";
import { ethers } from "ethers";

interface TransfersBoxProps {
    tx_id: number | null;
}

interface TransferResponse {
    transfers: TokenTransfer[];
}

const TransfersBox = (props: TransfersBoxProps) => {
    const [transfers, setTransfers] = useState<TransferResponse | null>(null);
    const config = useConfig();
    const { backendSettings } = useContext(BackendSettingsContext);

    React.useEffect(() => {
        console.log("Loading transfers for tx " + props.tx_id);
        const loadTransfers = async () => {
            if (props.tx_id) {
                const response = await backendFetch(backendSettings, `/transfers/${props.tx_id}`);
                const response_json = await response.json();
                setTransfers(response_json);
            }
        };

        loadTransfers().then(() => {
            console.log("Loaded transfers for tx " + props.tx_id);
        });
    }, [props.tx_id]);

    if (transfers === null) {
        return <div>Loading...</div>;
    }

    const transferCount = transfers.transfers.length ?? 0;
    let sum = BigInt(0);
    const distinctReceivers = new Set();
    for (let i = 0; i < transferCount; i++) {
        sum += BigInt(transfers.transfers[i].tokenAmount);
        distinctReceivers.add(transfers.transfers[i].receiverAddr);
    }
    const sumNum = sum.toString();
    if (transferCount === 0) {
        return (
            <div className={"transfers-box"}>
                <div className={"transfers-box-header"}>No transfers related to this transactions</div>
                <div className={"transfers-box-content"}></div>
            </div>
        );
    }
    const tokenAddr = transfers.transfers[0].tokenAddr;

    const chId = transfers.transfers[0].chainId;
    const chainId = typeof chId === "string" ? parseInt(chId) : chId;
    let tokenSymbol = "???";

    //console.log(config.chainSetup);
    if (tokenAddr == null) {
        tokenSymbol = config.chainSetup[chainId].currencyGasSymbol;
    } else if (config.chainSetup[chainId].glmAddress === tokenAddr) {
        tokenSymbol = config.chainSetup[chainId].currencyGlmSymbol;
    }

    //convert sumNum u256 to decimal
    const amount = ethers.utils.formatUnits(sumNum, 18);

    const row = (transfer: TokenTransfer, i: number) => {
        return <TransferBox key={i} transfer={transfer} tokenSymbol={tokenSymbol} />;
    };

    return (
        <div className={"transfers-box"}>
            <div className={"transfers-box-header"}>
                {transferCount} transfers to {distinctReceivers.size} distinct addresses for a sum of {amount}{" "}
                {tokenSymbol}:{" "}
            </div>
            <div className={"transfers-box-content"}>{transfers.transfers.map(row)}</div>
        </div>
    );
};

export default TransfersBox;
