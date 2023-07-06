import { DateTime } from "luxon";
import ChainTransfer from "./ChainTransfer";
import TransferIn from "./TransferIn";

interface BalanceEvent {
    id: string;
    date: DateTime;
    transferred: bigint;
    balance: bigint;
    chainTransfer: ChainTransfer | null;
    transferIn: TransferIn | null;
}

export default BalanceEvent;
