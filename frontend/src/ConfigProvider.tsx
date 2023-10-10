import React, { createContext, useContext, useEffect, useState } from "react";
import PaymentDriverConfig from "./model/PaymentDriverConfig";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { backendFetch } from "./common/BackendCall";

export let DEFAULT_BACKEND_URL = "";
export const FRONTEND_BASE = "/erc20/frontend/";

export function globalSetDefaultBackendUrl(backendUrl: string) {
    DEFAULT_BACKEND_URL = backendUrl;
}

export const ConfigContext = createContext<PaymentDriverConfig | null | string>(null);
export const useConfigOrNull = () => useContext<PaymentDriverConfig | null | string>(ConfigContext);
export const useConfig = () => {
    const value = useConfigOrNull();
    if (value == null || typeof value === "string") {
        throw new Error("Config not available");
    }
    return value;
};

interface ConfigProviderProps {
    children: React.ReactNode;
}

export const ConfigProvider = (props: ConfigProviderProps) => {
    const [config, setConfig] = useState<PaymentDriverConfig | null | string>(null);
    const { backendSettings } = useContext(BackendSettingsContext);

    useEffect(() => {
        (async () => {
            setConfig(`Connecting to ${backendSettings.backendUrl}`);
            let responseErr = null;
            let responseBody = null;
            try {
                const response = await backendFetch(backendSettings, "/config");
                if (response.type === "opaque") {
                    setConfig(`Failed to connect to ${backendSettings.backendUrl} due to CORS policy`);
                    return;
                }
                responseErr = response;
                responseBody = await response.text();
                const response_json = JSON.parse(responseBody);
                setConfig(response_json.config);
            } catch (_e) {
                console.log("Error fetching config", responseErr);
                if (responseBody) {
                    console.log("Response body: ", responseBody);
                }
                setConfig(`Failed to connect to ${backendSettings.backendUrl}`);
            }
        })();
    }, [setConfig, backendSettings]);

    return <ConfigContext.Provider value={config}>{props.children}</ConfigContext.Provider>;
};
