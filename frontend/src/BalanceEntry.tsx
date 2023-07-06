import React from "react";
import BalanceEvent from "./model/BalanceEvent";

interface BalanceEntryProps {
    event: BalanceEvent;
}

const BalanceEntry = (props: BalanceEntryProps) => {
    let title = "Unknown entry";
    if (props.event.transferIn !== null) {
        title = `Transfer in`;
    } else if (props.event.chainTransfer !== null) {
        title = `Chain transfer`;
    }

    return (
        <div>
            <span>
                {title} {props.event.date.toJSDate().toISOString()} {props.event.transferred.toString()} - balance:{" "}
                {props.event.balance.toString()}
            </span>
        </div>
    );
};

export default BalanceEntry;
