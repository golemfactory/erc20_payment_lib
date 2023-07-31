import React, { createContext, useCallback, useState } from "react";
import BackendSettings from "./common/BackendSettings";

interface BackendSettingsContextType {
    backendSettings: BackendSettings;
    setBackendSettings: (backendSettings: BackendSettings) => void;
    resetSettings: () => void;
}

export const BackendSettingsContext = createContext<BackendSettingsContextType>({
    backendSettings: {
        backendUrl: "",
        bearerToken: "",
        enableBearerToken: false,
    },
    setBackendSettings: (backendSettings: BackendSettings) => {
        console.error(`setBackendSettings not implemented: ${backendSettings}`);
    },
    resetSettings: () => {
        console.error("resetSettings not implemented");
    },
});

interface BackendSettingsProviderProps {
    children: React.ReactNode;
}

export const BackendSettingsProvider = (props: BackendSettingsProviderProps) => {
    const backendUrl = window.localStorage.getItem("backendUrl") ?? DEFAULT_BACKEND_URL;
    const bearerToken = window.localStorage.getItem("bearerToken") ?? "";
    const enableBearerToken = window.localStorage.getItem("bearerTokenEnabled") === "1" ?? false;

    const defaultBackendSettings = {
        backendUrl: backendUrl,
        bearerToken: bearerToken,
        enableBearerToken: enableBearerToken,
    };

    const [backendSettings, _setBackendSettings] = useState<BackendSettings>(defaultBackendSettings);
    const setBackendSettings = useCallback(
        (settings: BackendSettings) => {
            window.localStorage.setItem("backendUrl", settings.backendUrl);
            window.localStorage.setItem("bearerToken", settings.bearerToken);
            window.localStorage.setItem("bearerTokenEnabled", settings.enableBearerToken ? "1" : "0");
            _setBackendSettings(settings);
        },
        [_setBackendSettings],
    );

    const resetSettings = useCallback(() => {
        const newSettings = {
            backendUrl: DEFAULT_BACKEND_URL,
            bearerToken: "",
            enableBearerToken: false,
        };
        setBackendSettings(newSettings);
    }, [setBackendSettings]);

    return (
        <BackendSettingsContext.Provider value={{ backendSettings, setBackendSettings, resetSettings }}>
            {props.children}
        </BackendSettingsContext.Provider>
    );
};
