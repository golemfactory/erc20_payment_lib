import React, { useCallback, useContext, useEffect, useState } from "react";
import "./SingleCallInfo.css";
import { useParams } from "react-router";
import { backendFetch } from "./common/BackendCall";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { LatestCall } from "./LatestCalls";
import { JSONTree } from "react-json-tree";
import { KeyPath } from "react-json-tree/src/types";
import { Tab, Tabs, TabList, TabPanel } from "react-tabs";
import "react-tabs/style/react-tabs.css";
import DateBox from "./DateBox";

//interface SingleCallInfoProps {
//call: any;
//}

const theme = {
    scheme: "default",
    author: "chris kempson (http://chriskempson.com)",
    base00: "#181818",
    base01: "#282828",
    base02: "#383838",
    base03: "#585858",
    base04: "#b8b8b8",
    base05: "#d8d8d8",
    base06: "#e8e8e8",
    base07: "#f8f8f8",
    base08: "#ab4642",
    base09: "#dc9656",
    base0A: "#f7ca88",
    base0B: "#a1b56c",
    base0C: "#86c1b9",
    base0D: "#7cafc2",
    base0E: "#ba8baf",
    base0F: "#a16946",
};

const SingleCallInfo = (/*props: SingleCallInfoProps*/) => {
    const params = useParams();
    const { backendSettings } = useContext(BackendSettingsContext);
    const [call, setCall] = useState<LatestCall | null>(null);
    const callNoInt = params.callNo ? parseInt(params.callNo) : -1;
    const [callNo, setCallNo] = useState<number>(callNoInt);

    const loadCallInfo = useCallback(async () => {
        try {
            const response = await backendFetch(backendSettings, `/call/${params.key}/${callNo}`);
            const response_json = await response.json();
            setCall(response_json.call);
        } catch (e) {
            console.log(e);
            setCall(null);
        }
    }, [params, callNo]);

    useEffect(() => {
        loadCallInfo().then();
    }, [loadCallInfo]);

    let jsonRequest = null;
    try {
        jsonRequest = call?.request ? JSON.parse(call?.request) : null;
    } catch (e) {
        console.log(e);
    }
    let jsonResponse = null;
    try {
        jsonResponse = call?.response ? JSON.parse(call?.response) : null;
    } catch (e) {
        console.log(e);
    }

    if (callNo < 0) {
        return (
            <div className={"single-call-info"}>
                <h3>Call no is not specified</h3>
            </div>
        );
    }
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const shouldExpandNodeInitially = (keyName: KeyPath, data: unknown, level: number) => {
        return true;
    };

    function changeCall(nextCall: number) {
        setCallNo(nextCall);
        window.history.replaceState(null, "", `${FRONTEND_BASE}call/${params.key}/${nextCall}`);
    }

    return (
        <div className={"single-call-info"}>
            <button
                style={{ margin: "0 0.5rem 0.5rem 0" }}
                disabled={callNo == 0}
                onClick={() => changeCall(callNo - 1)}
            >
                Previous call
            </button>
            <button onClick={() => changeCall(callNo + 1)}>Next call</button>
            <table>
                <tbody>
                    <tr>
                        <th>Web3 endpoint key</th>
                        <td>{params.key}</td>
                    </tr>
                    <tr>
                        <th>Call no</th>
                        <td>{call?.id}</td>
                    </tr>
                    <tr>
                        <th>Call time</th>
                        <td>
                            <DateBox date={call?.date} title={"Call time"} minimal={false} />
                        </td>
                    </tr>
                    <tr>
                        <th>Response time</th>
                        <td>{call?.responseTime}</td>
                    </tr>
                </tbody>
            </table>
            <div>
                <h3>Request</h3>
                <Tabs>
                    <TabList>
                        <Tab>JSON</Tab>
                        <Tab>Raw</Tab>
                    </TabList>
                    <TabPanel>
                        {jsonRequest && (
                            <JSONTree
                                shouldExpandNodeInitially={shouldExpandNodeInitially}
                                theme={theme}
                                invertTheme={true}
                                data={jsonRequest}
                            />
                        )}
                    </TabPanel>
                    <TabPanel>
                        <code>{call?.request}</code>
                    </TabPanel>
                </Tabs>
            </div>
            <div>
                <h3>Response</h3>
                <Tabs>
                    <TabList>
                        <Tab>JSON</Tab>
                        <Tab>Raw</Tab>
                    </TabList>
                    <TabPanel selected={false}>
                        {jsonResponse && (
                            <JSONTree
                                shouldExpandNodeInitially={shouldExpandNodeInitially}
                                theme={theme}
                                invertTheme={true}
                                data={jsonResponse}
                            />
                        )}
                    </TabPanel>
                    <TabPanel selected={true}>
                        <code>{call?.response}</code>
                    </TabPanel>
                </Tabs>
            </div>
        </div>
    );
};

export default SingleCallInfo;
