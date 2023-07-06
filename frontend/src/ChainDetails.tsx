import React from "react";
import { useConfig } from "./ConfigProvider";
import ChainSetup from "./model/ChainSetup";
import { FiExternalLink } from "react-icons/fi";
import "./ChainDetails.css";

interface ChainDetailsProps {
    chainId: number | string;
}

const ChainDetails = (props: ChainDetailsProps) => {
    const config = useConfig();

    const chainId = typeof props.chainId === "string" ? parseInt(props.chainId) : props.chainId;
    if (chainId == null) {
        return <div>Unknown chain</div>;
    }
    const chainSetup: ChainSetup = config.chainSetup[chainId];
    if (!chainSetup) {
        return <span>No {chainId} in config</span>;
    }

    return (
        <a href={chainSetup.blockExplorerUrl} title={`chain id: ${props.chainId}`}>
            <div className={"chain-details-chain"}>
                <FiExternalLink className={"chain-details-chain-icon"} />
                <div className={"chain-details-chain-name"}>{chainSetup.chainName}</div>
            </div>
        </a>
    );
};

export default ChainDetails;
