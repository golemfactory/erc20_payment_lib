import React, { useCallback, useContext, useState } from "react";
import "./Endpoints.css";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { backendFetch } from "./common/BackendCall";
import { confirm } from "react-confirm-box";

interface EndpointProps {
    apikey: string;
}

interface EndpointProblems {
    timeoutChance: number;
    minTimeoutMs: number;
    maxTimeoutMs: number;
    errorChance: number;
    malformedResponseChance: number;
    skipSendingRawTransactionChance: number;
    sendTransactionButReportFailureChance: number;
    allowOnlyParsedCalls: boolean;
    allowOnlySingleCalls: boolean;
}

const Endpoint = (props: EndpointProps) => {
    React.useEffect(() => {
        console.log("Refreshing dashboard...");
        //timeout
        //sleep
    }, [props]);

    const [problems, setProblems] = useState<EndpointProblems | null>(null);
    const { backendSettings } = useContext(BackendSettingsContext);
    const [errorChance, setErrorChance] = useState<string>("");
    const [timeoutChance, setTimeoutChance] = useState<string>("");
    const [minTimeoutMs, setMinTimeoutMs] = useState<string>("");
    const [maxTimeoutMs, setMaxTimeoutMs] = useState<string>("");
    const [malformedResponseChance, setMalformedResponseChance] = useState<string>("");
    const [skipSendingRawTransactionChance, setSkipSendingRawTransactionChance] = useState<string>("");
    const [sendTransactionButReportFailureChance, setSendTransactionButReportFailureChance] = useState<string>("");

    const [errorChanceValidation, setErrorChanceValidation] = useState<string>("");
    const [timeoutChanceValidation, setTimeoutChanceValidation] = useState<string>("");
    const [minTimeoutMsValidation, setMinTimeoutMsChanceValidation] = useState<string>("");
    const [maxTimeoutMsValidation, setMaxTimeoutMsChanceValidation] = useState<string>("");
    const [malformedResponseChanceValidation, setMalformedResponseChanceValidation] = useState<string>("");
    const [skipSendingRawTransactionChanceValidation, setRawTransactionChanceValidation] = useState<string>("");
    const [sendTransactionButReportFailureChanceValidation, setSendTransactionButReportFailureChanceValidation] = useState<string>("");

    const [refresh, setRefresh] = useState(0);

    const loadProblems = useCallback(async () => {
        try {
            const response = await backendFetch(backendSettings, `/problems/${props.apikey}`);
            const response_json = await response.json();
            setProblems(response_json.problems);
            setErrorChance(response_json.problems.errorChance.toString());
            setTimeoutChance(response_json.problems.timeoutChance.toString());
            setMinTimeoutMs(response_json.problems.minTimeoutMs.toString());
            setMaxTimeoutMs(response_json.problems.maxTimeoutMs.toString());
            setMalformedResponseChance(response_json.problems.malformedResponseChance.toString());
            setSkipSendingRawTransactionChance(response_json.problems.skipSendingRawTransactionChance.toString());
            setSendTransactionButReportFailureChance(response_json.problems.sendTransactionButReportFailureChance.toString());
        } catch (e) {
            console.log(e);
            setProblems(null);
        }
    }, [setProblems, refresh]);
    React.useEffect(() => {
        loadProblems().then(() => {
            // loadProblems finished
        });
    }, [loadProblems]);

    React.useEffect(() => {
        const errorChanceNumber = parseFloat(errorChance);
        if (isNaN(errorChanceNumber)) {
            setErrorChanceValidation("Not a number");
        } else if (errorChanceNumber >= 0.0 && errorChanceNumber <= 1.0) {
            setErrorChanceValidation("");
        } else {
            setErrorChanceValidation("Has to be number between 0.0 and 1.0");
        }
    }, [errorChance]);
    React.useEffect(() => {
        const timeoutChanceNumber = parseFloat(timeoutChance);
        if (isNaN(timeoutChanceNumber)) {
            setTimeoutChanceValidation("Not a number");
        } else if (timeoutChanceNumber >= 0.0 && timeoutChanceNumber <= 1.0) {
            setTimeoutChanceValidation("");
        } else {
            setTimeoutChanceValidation("Has to be number between 0.0 and 1.0");
        }
    }, [timeoutChance]);
    React.useEffect(() => {
        const minTimeoutMsNumber = parseFloat(minTimeoutMs);
        if (isNaN(minTimeoutMsNumber)) {
            setMinTimeoutMsChanceValidation("Not a number");
        } else if (minTimeoutMsNumber >= 0 && minTimeoutMsNumber <= 100000) {
            setMinTimeoutMsChanceValidation("");
        } else {
            setMinTimeoutMsChanceValidation("Has to be number >= 0");
        }
    }, [minTimeoutMs]);
    React.useEffect(() => {
        const maxTimeoutMsNumber = parseFloat(maxTimeoutMs);
        if (isNaN(maxTimeoutMsNumber)) {
            setMaxTimeoutMsChanceValidation("Not a number");
        } else if (maxTimeoutMsNumber >= 0 && maxTimeoutMsNumber <= 100000) {
            setMaxTimeoutMsChanceValidation("");
        } else {
            setMaxTimeoutMsChanceValidation("Has to be number >= 0");
        }
    }, [maxTimeoutMs]);
    React.useEffect(() => {
        const malformedResponseChanceNumber = parseFloat(malformedResponseChance);
        if (isNaN(malformedResponseChanceNumber)) {
            setMalformedResponseChanceValidation("Not a number");
        } else if (malformedResponseChanceNumber >= 0.0 && malformedResponseChanceNumber <= 1.0) {
            setMalformedResponseChanceValidation("");
        } else {
            setMalformedResponseChanceValidation("Has to be number between 0.0 and 1.0");
        }
    }, [malformedResponseChance]);
    React.useEffect(() => {
        const skipSendingRawTransactionChanceNumber = parseFloat(skipSendingRawTransactionChance);
        if (isNaN(skipSendingRawTransactionChanceNumber)) {
            setRawTransactionChanceValidation("Not a number");
        } else if (skipSendingRawTransactionChanceNumber >= 0.0 && skipSendingRawTransactionChanceNumber <= 1.0) {
            console.log("Updating error chance to " + skipSendingRawTransactionChanceNumber);
            setRawTransactionChanceValidation("");
        } else {
            setRawTransactionChanceValidation("Has to be number between 0.0 and 1.0");
        }
    }, [skipSendingRawTransactionChance]);
    React.useEffect(() => {
        const sendTransactionButReportFailureChanceNumber = parseFloat(sendTransactionButReportFailureChance);
        if (isNaN(sendTransactionButReportFailureChanceNumber)) {
            setSendTransactionButReportFailureChanceValidation("Not a number");
        } else if (sendTransactionButReportFailureChanceNumber >= 0.0 && sendTransactionButReportFailureChanceNumber <= 1.0) {
            console.log("Updating error chance to " + sendTransactionButReportFailureChanceNumber);
            setSendTransactionButReportFailureChanceValidation("");
        } else {
            setSendTransactionButReportFailureChanceValidation("Has to be number between 0.0 and 1.0");
        }
    }, [sendTransactionButReportFailureChance]);
    const saveProblems = useCallback(async () => {
        if (problems) {
            console.log("Saving problems");
            const newProblems: EndpointProblems = {
                errorChance: parseFloat(errorChance),
                timeoutChance: parseFloat(timeoutChance),
                minTimeoutMs: parseFloat(minTimeoutMs),
                maxTimeoutMs: parseFloat(maxTimeoutMs),
                malformedResponseChance: parseFloat(malformedResponseChance),
                skipSendingRawTransactionChance: parseFloat(skipSendingRawTransactionChance),
                sendTransactionButReportFailureChance: parseFloat(sendTransactionButReportFailureChance),
                allowOnlyParsedCalls: problems.allowOnlyParsedCalls,
                allowOnlySingleCalls: problems.allowOnlySingleCalls,
            };
            await backendFetch(backendSettings, `/problems/set/${props.apikey}`, {
                method: "POST",
                body: JSON.stringify(newProblems),
            });

            setRefresh(refresh + 1);
        }
    }, [
        problems,
        refresh,
        setRefresh,
        errorChance,
        timeoutChance,
        minTimeoutMs,
        maxTimeoutMs,
        malformedResponseChance,
        skipSendingRawTransactionChance,
        sendTransactionButReportFailureChance,
    ]);

    const deleteEndpoint = useCallback(async () => {
        const result = await confirm("Are you sure you want to delete all endpoint history?");
        if (result) {
            await backendFetch(backendSettings, `/keys/delete/${props.apikey}`, {
                method: "POST",
            });
            setRefresh(refresh + 1);
        }
    }, [refresh, setRefresh, props]);

    if (problems === null) {
        return (
            <div className={"endpoint"}>
                <div>loading...</div>
            </div>
        );
    }

    let buttonDisabled =
        errorChanceValidation !== "" ||
        timeoutChanceValidation !== "" ||
        minTimeoutMsValidation !== "" ||
        maxTimeoutMsValidation !== "" ||
        malformedResponseChanceValidation !== "" ||
        skipSendingRawTransactionChanceValidation !== "" ||
        sendTransactionButReportFailureChanceValidation !== "";

    if (
        errorChance === problems.errorChance.toString() &&
        timeoutChance === problems.timeoutChance.toString() &&
        minTimeoutMs === problems.minTimeoutMs.toString() &&
        maxTimeoutMs === problems.maxTimeoutMs.toString() &&
        malformedResponseChance === problems.malformedResponseChance.toString() &&
        skipSendingRawTransactionChance === problems.skipSendingRawTransactionChance.toString() &&
        sendTransactionButReportFailureChance === problems.sendTransactionButReportFailureChance.toString()
    ) {
        buttonDisabled = true;
    }

    return (
        <div className={"endpoint"}>
            <div className={"endpoint-header-title"}>Endpoint {props.apikey}</div>
            <div>{JSON.stringify(problems)}</div>

            <table>
                <tbody>
                    <tr>
                        <th>Error chance per request</th>
                        <td>
                            <input value={errorChance} onChange={(e) => setErrorChance(e.target.value)} />
                        </td>
                        <td>{problems.errorChance}</td>
                        <td>
                            <div>{errorChanceValidation}</div>
                        </td>
                    </tr>
                    <tr>
                        <th>Timeout chance per request</th>
                        <td>
                            <input value={timeoutChance} onChange={(e) => setTimeoutChance(e.target.value)} />
                        </td>
                        <td>{problems.timeoutChance}</td>
                        <td>
                            <div>{timeoutChanceValidation}</div>
                        </td>
                    </tr>
                    <tr>
                        <th>Minimal timeout in ms</th>
                        <td>
                            <input value={minTimeoutMs} onChange={(e) => setMinTimeoutMs(e.target.value)} />
                        </td>
                        <td>{problems.minTimeoutMs}</td>
                        <td>
                            <div>{minTimeoutMsValidation}</div>
                        </td>
                    </tr>
                    <tr>
                        <th>Maximum timeout in ms</th>
                        <td>
                            <input value={maxTimeoutMs} onChange={(e) => setMaxTimeoutMs(e.target.value)} />
                        </td>
                        <td>{problems.maxTimeoutMs}</td>
                        <td>
                            <div>{maxTimeoutMsValidation}</div>
                        </td>
                    </tr>
                    <tr>
                        <th>Malformed response chance</th>
                        <td>
                            <input
                                value={malformedResponseChance}
                                onChange={(e) => setMalformedResponseChance(e.target.value)}
                            />
                        </td>
                        <td>{problems.malformedResponseChance}</td>
                        <td>
                            <div>{malformedResponseChanceValidation}</div>
                        </td>
                    </tr>
                    <tr>
                        <th>Skip sending chance</th>
                        <td>
                            <input
                                value={skipSendingRawTransactionChance}
                                onChange={(e) => setSkipSendingRawTransactionChance(e.target.value)}
                            />
                        </td>
                        <td>{problems.skipSendingRawTransactionChance}</td>
                        <td>
                            <div>{skipSendingRawTransactionChanceValidation}</div>
                        </td>
                    </tr>
                    <tr>
                        <th>Send but report error chance</th>
                        <td>
                            <input
                                value={sendTransactionButReportFailureChance}
                                onChange={(e) => setSendTransactionButReportFailureChance(e.target.value)}
                            />
                        </td>
                        <td>{problems.sendTransactionButReportFailureChance}</td>
                        <td>
                            <div>{sendTransactionButReportFailureChanceValidation}</div>
                        </td>
                    </tr>
                </tbody>
            </table>
            <button onClick={() => deleteEndpoint()}>Delete</button>
            <button disabled={buttonDisabled} onClick={() => saveProblems()}>
                Save
            </button>
        </div>
    );
};
const Endpoints = () => {
    const [keys, setKeys] = useState<string[]>([]);
    const [refresh, setRefresh] = useState(0);

    const { backendSettings } = useContext(BackendSettingsContext);
    const loadEndpoints = useCallback(async () => {
        try {
            const response = await backendFetch(backendSettings, `/keys`);
            const response_json = await response.json();
            setKeys(response_json.keys);
        } catch (e) {
            console.log(e);
            setKeys([]);
        }
    }, [setKeys]);
    React.useEffect(() => {
        console.log("Refreshing dashboard...");
        //timeout
        //sleep
        loadEndpoints().then(() => {
            //setLoading(false);
        });
    }, [loadEndpoints, refresh]);

    const deleteAll = useCallback(async () => {
        const result = await confirm("Are you sure you want to delete all endpoints history?");
        if (result) {
            await backendFetch(backendSettings, `/keys/delete_all`, {
                method: "POST",
            });
            setRefresh(refresh + 1);
        }
    }, [refresh, setRefresh]);

    const refreshAll = useCallback(async () => {
        setRefresh(refresh + 1);
        setKeys([]);
    }, [refresh, setRefresh]);

    function row(key: string) {
        return <Endpoint key={key} apikey={key} />;
    }
    return (
        <div className={"endpoints"}>
            <div className={"endpoints-header"}>
                <button onClick={() => refreshAll()}>Refresh</button>
                <button onClick={() => deleteAll()}>Delete All Endpoints</button>
            </div>
            {keys.map(row)}
        </div>
    );
};

export default Endpoints;
