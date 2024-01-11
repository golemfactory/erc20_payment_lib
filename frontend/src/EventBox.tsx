import React, { useContext } from "react";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import useWebSocket from "react-use-websocket";
import "./EventBox.css";

interface EventBoxProps {
    selectedChain: string | null;
}

interface NoGasDetails {
    tx: any;
    gasBalance: string;
    gasNeeded: string;
}

interface NoTokenDetails {
    tx: any;
    sender: string;
    tokenBalance: string;
    tokenNeeded: string;
}

interface GasLowInfo {
    tx: any;
    txMaxFeePerGasGwei: string;
    blockDate: number;
    blockNumber: number;
    blockBaseFeePerGasGwei: string;
    assumedMinPriorityFeeGwei: string;
    userFriendlyMessage: string;
}

interface TransactionStuck {
    noGas?: NoGasDetails;
    noToken?: NoTokenDetails;
    gasPriceLow?: GasLowInfo;
    rpcEndpointProblems?: string;
}
interface BalanceEventContent {
    transactionStuck?: TransactionStuck;
}
interface BalanceEvent {
    createDate: string;
    content: BalanceEventContent;
}

const EventBox = (_props: EventBoxProps) => {
    const { backendSettings } = useContext(BackendSettingsContext);
    const [events, setEvents] = React.useState<BalanceEvent[]>([]);

    useWebSocket(backendSettings.backendUrl.replace("http://", "ws://") + "/event_stream", {
        onOpen: () => {
            console.log("WebSocket connection established.");
        },
        onMessage: (event) => {
            console.log("Received event: ", event);
            setEvents((events) => [...events, JSON.parse(event.data)]);
        },
        onError: (event) => {
            console.error("WebSocket error: ", event);
        },
        onClose: () => {
            console.log("WebSocket connection closed.");
        },
    });

    const row = (tx: BalanceEvent) => {
        if (tx && tx.content && tx.content.transactionStuck) {
            if (tx.content.transactionStuck.noToken) {
                const noToken = tx.content.transactionStuck.noToken;

                return (
                    <div>
                        No token: {tx.createDate} sender: {noToken.sender} token balance: {noToken.tokenBalance} needed:{" "}
                        {noToken.tokenNeeded}
                    </div>
                );
            }
        }
        return <div>Unknown event: {tx.createDate}</div>;
    };

    return (
        <div>
            Event Stream 2<div>{events.map(row)}</div>
        </div>
    );
};

export default EventBox;
