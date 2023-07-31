import React, { useContext } from "react";
import "./WelcomePage.css";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { useBackendConfig } from "./ConfigProvider";

const WelcomePage = () => {
    const { backendSettings } = useContext(BackendSettingsContext);
    const config = useBackendConfig();

    return (
        <div className="welcome-page">
            <h1>Web3 proxy</h1>
            <p>Connected to the endpoint {backendSettings.backendUrl}</p>
            <p>Frontend version {APP_VERSION}</p>
            <p>Backend version {config.version}</p>
        </div>
    );
};

export default WelcomePage;
