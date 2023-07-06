import React from "react";
import "./TransferBox.css";
import ContractDetails from "./ContractDetails";
import { fromWei } from "./common/Web3Utils";
import TokenTransfer from "./model/TokenTransfer";

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
                {fromWei(transfer.tokenAmount)} {props.tokenSymbol}
            </div>
        </div>
    );
};

export default TransferBox;
