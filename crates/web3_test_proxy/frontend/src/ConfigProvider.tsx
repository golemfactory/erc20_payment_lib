import React, { createContext, useContext, useEffect, useState } from "react";
import BackendConfig from "./common/BackendConfig";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { backendFetch } from "./common/BackendCall";

class BackendConfigResult {
    config: BackendConfig | null;
    progress: string;
    error: string | null;

    constructor(config: BackendConfig | null, progress: string, error: string | null) {
        this.config = config;
        this.progress = progress;
        this.error = error;
    }
}

export const ConfigContext = createContext<BackendConfigResult>(new BackendConfigResult(null, "", null));
export const useConfigResult = () => useContext<BackendConfigResult>(ConfigContext);
export function useBackendConfig(): BackendConfig {
    const value = useConfigResult();
    if (value.config == null) {
        throw new Error("Config not available");
    }
    return value.config;
}

interface ConfigProviderProps {
    children: React.ReactNode;
}

export const ConfigProvider = (props: ConfigProviderProps) => {
    const [config, setConfig] = useState<BackendConfigResult>(new BackendConfigResult(null, "", null));
    const { backendSettings } = useContext(BackendSettingsContext);

    useEffect(() => {
        (async () => {
            const configUrl = backendSettings.backendUrl + "/config";
            setConfig(new BackendConfigResult(null, `Connecting to ${configUrl}`, null));
            let responseBody = null;
            try {
                const response = await backendFetch(backendSettings, "/config");
                if (response.type === "opaque") {
                    setConfig(
                        new BackendConfigResult(null, "", `Failed to connect to ${configUrl} due to CORS policy`),
                    );
                    return;
                }
                responseBody = await response.text();
                const response_json = JSON.parse(responseBody);
                if (!response_json["config"]) {
                    setConfig(new BackendConfigResult(null, "", `No config field found on endpoint ${configUrl} `));
                    return;
                }
                setConfig(new BackendConfigResult(response_json.config, "", null));
            } catch (e) {
                console.log("Error fetching config", e);
                if (responseBody) {
                    console.log("Response body: ", responseBody);
                }
                setConfig(new BackendConfigResult(null, "", `Failed to connect to ${configUrl}`));
            }
        })();
    }, [setConfig, backendSettings]);

    return <ConfigContext.Provider value={config}>{props.children}</ConfigContext.Provider>;
};
