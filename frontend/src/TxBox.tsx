import React, { useContext } from "react";
import "./TxBox.css";
import DateBox from "./DateBox";
import TransfersBox from "./TransfersBox";
import { Circles } from "react-loading-icons";
import { BiTime, BiError } from "react-icons/bi";
import { GiConfirmed } from "react-icons/gi";
import ChainDetails from "./ChainDetails";
import ContractDetails from "./ContractDetails";
import TransactionDetails from "./TransactionDetails";
import { fromWei } from "./common/Web3Utils";
import Web3Transaction from "./model/Web3Transaction";

import { BackendSettingsContext } from "./BackendSettingsProvider";
import { backendFetch } from "./common/BackendCall";

interface TransactionProps {
    tx_id: number | null;
    tx: Web3Transaction;
}

const TxBox = (props: TransactionProps) => {
    //let [tx, setTx] = React.useState(null);
    const { backendSettings } = useContext(BackendSettingsContext);

    /*const loadTxDetails = async () => {
        if (props.tx_id) {
            const response = await backendFetch(backendSettings, `/tx/${props.tx_id}`);
            const response_json = await response.json();

            setTx(response_json.tx);
        }
    }*/

    /*const loadData = async () => {
        for (let loopNo = 0; ; loopNo++) {
            loadTxDetails().then(() => {
            });
            await new Promise(r => setTimeout(r, 2000));
        }
    }*/

    /*React.useEffect(() => {
        loadData().then(() => {
        });
    }, [props.tx_id])*/

    const tx = props.tx;
    if (tx == null) {
        return <div className={"tx-container-wrapper"}></div>;
    }

    let feePaid: string | null = null;

    if (tx.feePaid) {
        feePaid = fromWei(tx.feePaid);
    }

    function SkipTx() {
        backendFetch(backendSettings, `/tx/skip/${props.tx_id}`, { method: "POST" }).then((resp) => {
            console.log(resp);
        });
    }

    function GetIcon() {
        if (tx.confirmDate != null && tx.error == null && tx.engineError == null) {
            return <GiConfirmed className={"tx-ok-icon"} />;
        } else if (tx.engineError != null) {
            return <BiError className={"tx-error-icon"} />;
        } else if (tx.error != null) {
            return <BiError className={"tx-error-icon"} />;
        } else if (tx.engineMessage) {
            return <Circles className={"tx-loading"} fill="#000000" />;
        } else {
            return <BiTime className={"tx-wait-icon"} />;
        }
    }
    function GetActions() {
        if (tx.engineMessage) {
            return <button onClick={SkipTx}>Skip transaction</button>;
        } else {
            return <></>;
        }
    }
    return (
        <div className={"tx-container-wrapper"}>
            <div className={"tx-container-header"}></div>

            <div className={"tx-container"}>
                <div className={"tx-id"}>db id: {tx.id}</div>
                <div className={"chain-id"}>
                    <ChainDetails chainId={tx.chainId} />
                </div>
                <div className={"tx-method"} title={"method"}>
                    {tx.method}
                </div>
                <div className={"tx-created"}>
                    <DateBox title="queued" date={tx.createdDate} />
                </div>
                <div className={"tx-from"}>
                    <ContractDetails isAddress={true} chainId={tx.chainId} contractAddress={tx.fromAddr} />
                </div>
                <div className={"tx-from-descr"}>From</div>
                <div className={"tx-to"}>
                    <ContractDetails isAddress={false} chainId={tx.chainId} contractAddress={tx.toAddr} />
                </div>
                <div className={"tx-to-descr"}>To</div>

                <div className={"tx-broadcast"}>
                    <DateBox title="broadcast" date={tx.broadcastDate} />
                </div>
                <div className={"tx-confirmed"}>
                    <DateBox title="confirmed" date={tx.confirmDate} />
                </div>
                {tx.txHash ? (
                    <div className={"tx-hash"}>
                        <span></span>
                        <TransactionDetails chainId={tx.chainId} transactionHash={tx.txHash} />
                    </div>
                ) : (
                    <div className={"tx-hash tx-hash-unknown"}>Tx hash - not available</div>
                )}
                {tx.nonce ? (
                    <div className={"tx-nonce"}>nonce: {tx.nonce}</div>
                ) : (
                    <div className={"tx-nonce tx-nonce-unknown"}>nonce: N/A</div>
                )}
                {tx.gasLimit ? (
                    <div className={"tx-gas-limit"}>gas limit: {tx.gasLimit}</div>
                ) : (
                    <div className={"tx-gas-limit tx-gas-limit-unknown"}>gas limit: N/A</div>
                )}
                {tx.feePaid ? (
                    <div className={"tx-fee"}>fee paid: {feePaid}</div>
                ) : (
                    <div className={"tx-fee tx-fee-unknown"}>fee paid: N/A</div>
                )}

                <div className={"tx-processing"}>{GetIcon()}</div>
                <div className={"tx-message"}>
                    {GetActions()}
                    {tx.engineMessage}{" "}
                    <span className={"tx-message-error"}>
                        {tx.engineError} {tx.error}
                    </span>
                </div>
            </div>
            <div className={"tx-transfers"}>
                <TransfersBox tx_id={tx.id} />
            </div>
        </div>
    );
};

export default TxBox;
