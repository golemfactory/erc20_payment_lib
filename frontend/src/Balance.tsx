import React, { useCallback, useContext } from "react";
import { useParams } from "react-router";
import ChainTransfer from "./model/ChainTransfer";
import TransferIn from "./model/TransferIn";
import { DateTime } from "luxon";
import BalanceEvent from "./model/BalanceEvent";
import BalanceEntry from "./BalanceEntry";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { backendFetch } from "./common/BackendCall";

const Balance = () => {
    //const [account, setAccount] = React.useState(null);
    const { account } = useParams();
    //const [payments, setPayments] = React.useState(null);
    const [events, setEvents] = React.useState<BalanceEvent[] | null>(null);
    const { backendSettings } = useContext(BackendSettingsContext);

    const loadAccountDetails = useCallback(async () => {
        const response = await backendFetch(backendSettings, `/account/${account}/in`);
        const response_json = await response.json();

        const chainTransfers: ChainTransfer[] = response_json.chainTransfers;
        const transfersIn: TransferIn[] = response_json.transfersIn;

        const events: BalanceEvent[] = [];
        for (const chainTransfer of chainTransfers) {
            const event: BalanceEvent = {
                id: `chain_transfer_${chainTransfer.id}`,
                date: DateTime.fromISO(chainTransfer.blockchainDate),
                chainTransfer: chainTransfer,
                transferIn: null,
                transferred: BigInt(chainTransfer.tokenAmount),
                balance: BigInt(0),
            };
            events.push(event);
        }
        for (const transferIn of transfersIn) {
            const event: BalanceEvent = {
                id: `transfer_in_${transferIn.id}`,
                date: DateTime.fromISO(transferIn.requestedDate),
                chainTransfer: null,
                transferIn: transferIn,
                transferred: -BigInt(transferIn.tokenAmount),
                balance: BigInt(0),
            };
            events.push(event);
        }
        events.sort((a, b) => a.date.toMillis() - b.date.toMillis());

        //drop chain transfers before the first transfer in
        const firstTransferIn = events.find((e) => e.transferIn != null);
        if (firstTransferIn != null) {
            const firstTransferInIndex = events.indexOf(firstTransferIn);
            events.splice(0, firstTransferInIndex);
        }

        let balance = BigInt(0);
        for (const event of events) {
            balance += event.transferred;
            event.balance = balance;
        }

        events.reverse();
        setEvents(events);
    }, [account, setEvents]);

    React.useEffect(() => {
        loadAccountDetails().then();
    }, [loadAccountDetails]);

    function row(event: BalanceEvent) {
        return <BalanceEntry key={event.id} event={event} />;
    }

    if (events === null) {
        return <div>Loading...</div>;
    }
    return (
        <div>
            <h1>Balance</h1>

            {events.map(row)}
        </div>
    );
};

export default Balance;
