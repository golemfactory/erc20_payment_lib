import React from "react";
import { useConfig } from "./ConfigProvider";
import ChainSetup from "./model/ChainSetup";
import { FiExternalLink } from "react-icons/fi";
import "./ContractDetails.css";

interface ContractDetailsProps {
    chainId: number | string;
    contractAddress: string | null;
    isAddress: boolean | string;
}

const ContractDetails = (props: ContractDetailsProps) => {
    const config = useConfig();

    const chainId = typeof props.chainId === "string" ? parseInt(props.chainId) : props.chainId;
    const chainSetup: ChainSetup = config.chainSetup[chainId];
    if (!chainSetup) {
        return <span>No {chainId} in config</span>;
    }
    let contractString = props.contractAddress;
    let contractTitle = `contract id: ${props.contractAddress}`;
    if (props.isAddress) {
        contractTitle = `address id: ${props.contractAddress}`;
    }
    //todo - fix this hack in more elegant way
    if (props.isAddress === "Receiver id") {
        contractTitle = `Receiver id: ${props.contractAddress}`;
    }
    if (chainSetup.multiContractAddress === props.contractAddress) {
        contractString = "Multi payment contract";
    }
    if (chainSetup.glmAddress === props.contractAddress) {
        contractString = `${chainSetup.currencyGlmSymbol} token`;
    }

    const url = `${chainSetup.blockExplorerUrl}/address/${props.contractAddress}`;

    return (
        <a href={url} title={contractTitle}>
            <div className={"contract-details-contract"}>
                <FiExternalLink className={"contract-details-contract-icon"} />

                <div className={"contract-details-contract-name"}>{contractString}</div>
            </div>
        </a>
    );
};

export default ContractDetails;
