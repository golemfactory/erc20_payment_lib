import React, { useCallback, useContext, useEffect } from "react";
import "./LatestCalls.css";
import { backendFetch } from "./common/BackendCall";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import DateBox from "./DateBox";

interface ParsedCall {
    method: string;
    address: string | null;
    to: string | null;
}
interface ParsedRequest {
    method: string;
    parsedCall: ParsedCall | null;
    params: string[];
}
export interface LatestCall {
    id: string;
    date: string;
    parsedRequest: ParsedRequest[];
    request: string;
    response: string;
    responseTime: number;
    statusCode: number;
}
interface LatestCalls {
    error: string | null;
    calls: LatestCall[] | null;
}

interface CellBoxProps {
    latestCall: LatestCall;
    endpointKey: string;
}

const CallBox = (props: CellBoxProps) => {
    const call = props.latestCall;
    return (
        <div className={"call-box"}>
            <div className={"call-box-header"}>
                <a href={`${FRONTEND_BASE}call/${props.endpointKey}/${call.id}`}>{call.id}</a>
            </div>
            <DateBox date={call.date} title={"Time"} minimal={true}></DateBox>
            <div className={"call-box-body"}>
                {call.parsedRequest.length > 0 ? (
                    <>
                        <div>{call.parsedRequest[0].method ?? "unknown"}</div>
                        <>
                            {call.parsedRequest[0].parsedCall?.to && (
                                <div>
                                    <div>Contract :</div>
                                    <div>{call.parsedRequest[0].parsedCall.to}</div>
                                </div>
                            )}
                            {call.parsedRequest[0].parsedCall && (
                                <div>
                                    <div>ERC20 balance:</div>
                                    <div>{call.parsedRequest[0].parsedCall.address}</div>
                                </div>
                            )}
                        </>
                    </>
                ) : (
                    <div>unknown</div>
                )}
            </div>
        </div>
    );
};

interface LatestCallsProps {
    apikey: string;
    refreshToken: number;
    showAtOnce: number;
}

const LatestCalls = (props: LatestCallsProps) => {
    const [calls, setCalls] = React.useState<LatestCalls | null>(null);
    const { backendSettings } = useContext(BackendSettingsContext);

    const loadTxCount = useCallback(async () => {
        try {
            const response = await backendFetch(backendSettings, `/calls/${props.apikey}/${props.showAtOnce}`);
            const response_json = await response.json();
            setCalls(response_json);
        } catch (e) {
            console.log(e);
            setCalls(null);
        }
    }, [props.refreshToken]);

    function row(latestCall: LatestCall) {
        return <CallBox key={latestCall.id} endpointKey={props.apikey} latestCall={latestCall} />;
    }

    useEffect(() => {
        loadTxCount().then();
    }, [loadTxCount]);

    if (calls === null) {
        return (
            <div className={"latest-calls-box"}>
                <div>
                    <h3>{props.apikey}</h3>
                </div>
                Loading...
                <hr />
            </div>
        );
    }
    if (calls.error) {
        return (
            <div className={"latest-calls-box"}>
                <div>
                    <h3>{props.apikey}</h3>
                </div>
                {calls.error}
                <hr />
            </div>
        );
    }
    return (
        <div className={"latest-calls-box"}>
            <div>
                <h3>{props.apikey}</h3>
            </div>
            {calls?.calls?.map(row)}
            <hr />
        </div>
    );
};

export default LatestCalls;
