import React, { useCallback, useContext, useState } from "react";
import "./Monitor.css";
import LatestCalls from "./LatestCalls";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { backendFetch } from "./common/BackendCall";

const Monitor = () => {
    const [nextRefresh, setNextRefresh] = useState(0);
    const { backendSettings } = useContext(BackendSettingsContext);
    const [keys, setKeys] = useState<string[]>([]);
    const [keysFromServer, setKeysFromServer] = useState<string[]>([]);
    const [filter, setFilter] = useState("");
    const [showAtOnce, setShowAtOnce] = useState("10");

    const loadActiveKeys = useCallback(async () => {
        try {
            let keysManual: string[] = [];
            if (filter.length > 0) {
                keysManual = filter.split(",").map((s) => s.trim());
                //remove empty strings
                keysManual = keysManual.filter((s) => s.length > 0);
            }
            //if (!keysManual) {
                const response = await backendFetch(backendSettings, `/keys/active`);
                const response_json = await response.json();
                let keys = response_json.keys.splice(0).sort();
                setKeysFromServer(keys);
                if (keysManual.length > 0) {
                    //check if keysManual is a subset of keys
                    keys = keysManual.filter((k: string) => keys.indexOf(k) >= 0);
                }
                setKeys(keys);

            //} else {
              //  setKeys(keysManual)
            //}
        } catch (e) {
            console.log(e);
            setKeys([]);
        }
    }, [filter, setKeys]);

    React.useEffect(() => {
        console.log("Refreshing dashboard...");
        //timeout
        //sleep
        loadActiveKeys().then(() => {
            setTimeout(() => {
                setNextRefresh(nextRefresh + 1);
            }, 2000);
        });
    }, [setNextRefresh, nextRefresh]);

    function row(key: string) {


        return <LatestCalls key={key} apikey={key} refreshToken={nextRefresh} showAtOnce={parseInt(showAtOnce) || 10} />;
    }

    return <div>
        <div className={"monitor-filter"}>
            <div className={"monitor-filter-row1"}>
                <div className="monitor-filter-el1" >Columns from server:</div>
                <div>{keysFromServer.join(",")}</div>
            </div>
            <div className="monitor-filter-row2">
                <div className="monitor-filter-el1" >Selected columns:</div>
                <input className="monitor-filter-row2-el2" type="text" value={filter} onChange={(e) => setFilter(e.target.value)} />
            </div>
            <div className="monitor-filter-row3">
                <div className="monitor-filter-el1" >Last displayed calls:</div>
                <input className="monitor-filter-row3-el2" type="text" value={showAtOnce} onChange={(e) => setShowAtOnce(e.target.value)} />
            </div>
        </div>


        <div className={"monitor-appkey-lister"}>{keys.map(row)}</div>
    </div>;
};

export default Monitor;
