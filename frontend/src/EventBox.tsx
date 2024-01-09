import React, { useContext } from "react";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import useWebSocket from "react-use-websocket";
import "./EventBox.css";

interface EventBoxProps {
    selectedChain: string | null;
}

interface BalanceEvent {
    text: string;
}

const EventBox = (_props: EventBoxProps) => {
    const { backendSettings } = useContext(BackendSettingsContext);
    const [events, _setEvents] = React.useState<BalanceEvent[]>([]);

    useWebSocket(backendSettings.backendUrl.replace("http://", "ws://") + "/event_stream", {
        onOpen: () => {
            console.log("WebSocket connection established.");
        },
        onMessage: (event) => {
            console.log("Received event: ", event);
        },
        onError: (event) => {
            console.error("WebSocket error: ", event);
        },
        onClose: () => {
            console.log("WebSocket connection closed.");
        },
    });

    return (
        <div>
            Event Stream
            <div>
                {events.map((event, i) => (
                    <div key={i}>{event.text}</div>
                ))}
            </div>
        </div>
    );
};

export default EventBox;
