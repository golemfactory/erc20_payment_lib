import React from "react";
import "./TransferBox.css";
import ContractDetails from "./ContractDetails";
import TokenTransfer from "./model/TokenTransfer";
import { ethers } from "ethers";

interface TransferBoxProps {
    transfer: TokenTransfer;
    tokenSymbol: string;
}

const TransferBox = (props: TransferBoxProps) => {
    const transfer = props.transfer;
    return (
        <div className={"transfer-box"}>
            <div className={"transfer-id"} title={"Transfer db id"}>
                {transfer.id}
            </div>
            <div className={"transfer-receiver"} title={"Receiver address"}>
                <ContractDetails
                    contractAddress={transfer.receiverAddr}
                    chainId={transfer.chainId}
                    isAddress={"Receiver id"}
                />
            </div>
            <div className={"transfer-token"} title={"Tokens transferred"}>
                {ethers.utils.formatUnits(transfer.tokenAmount, 18)} {props.tokenSymbol}
            </div>
        </div>
    );
};

export default TransferBox;
