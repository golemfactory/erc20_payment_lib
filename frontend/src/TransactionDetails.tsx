import React from "react";
import { useConfig } from "./ConfigProvider";
import ChainSetup from "./model/ChainSetup";
import { FiExternalLink } from "react-icons/fi";
import "./TransactionDetails.css";

interface TransactionDetailsProps {
    chainId: string | number;
    transactionHash: string | null;
}

const TransactionDetails = (props: TransactionDetailsProps) => {
    const config = useConfig();

    const chainId = typeof props.chainId === "string" ? parseInt(props.chainId) : props.chainId;
    const chainSetup: ChainSetup = config.chainSetup[chainId];
    if (!chainSetup) {
        return <span>No {chainId} in config</span>;
    }
    const url = `${chainSetup.blockExplorerUrl}/tx/${props.transactionHash}`;

    return (
        <a href={url} title={`Transaction hash: ${props.transactionHash}`}>
            <div className={"transaction-details-transaction"}>
                <FiExternalLink className={"transaction-details-transaction-icon"} />

                <div className={"transaction-details-transaction-name"}>{props.transactionHash}</div>
            </div>
        </a>
    );
};

export default TransactionDetails;
