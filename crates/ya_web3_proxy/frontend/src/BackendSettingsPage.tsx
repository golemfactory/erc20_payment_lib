import React, { useContext, useEffect } from "react";
import "./BackendSettingsPage.css";
import { BackendSettingsContext } from "./BackendSettingsProvider";

const BackendSettingsPage = () => {
    const { backendSettings, setBackendSettings, resetSettings } = useContext(BackendSettingsContext);
    //const backendSettings = props.backendSettings;

    const [backendUrl, setBackendUrl] = React.useState(backendSettings.backendUrl);
    const backendChanged = (e: React.ChangeEvent<HTMLInputElement>) => {
        setBackendUrl(e.target.value);
    };
    const [bearerToken, setBearerToken] = React.useState(backendSettings.bearerToken);
    const [enableBearerToken, setEnableBearerToken] = React.useState(backendSettings.enableBearerToken);

    const bearerEnabledChanged = (e: React.ChangeEvent<HTMLInputElement>) => {
        setEnableBearerToken(e.target.checked);
    };
    const bearerChanged = (e: React.ChangeEvent<HTMLInputElement>) => {
        setBearerToken(e.target.value);
    };
    const saveAndCheck = () => {
        //
        const newSettings = {
            backendUrl: backendUrl,
            bearerToken: bearerToken,
            enableBearerToken: enableBearerToken,
        };
        setBackendSettings(newSettings);
    };

    const cancelChanges = () => {
        setBackendUrl(backendSettings.backendUrl);
        setBearerToken(backendSettings.bearerToken);
        setEnableBearerToken(backendSettings.enableBearerToken);
    };

    useEffect(() => {
        setBackendUrl(backendSettings.backendUrl);
        setBearerToken(backendSettings.bearerToken);
        setEnableBearerToken(backendSettings.enableBearerToken);
    }, [backendSettings]);

    const resetToDefault = () => {
        resetSettings();
    };

    const isCancelEnabled = () => {
        return (
            backendUrl !== backendSettings.backendUrl ||
            bearerToken !== backendSettings.bearerToken ||
            enableBearerToken !== backendSettings.enableBearerToken
        );
    };

    return (
        <div className={"backend-settings"}>
            <div>Backend settings</div>
            <hr />
            <h3>Backend URL:</h3>
            <input type="text" value={backendUrl} onChange={backendChanged} />
            <hr />
            <h3>Backend security:</h3>
            <p>
                <span style={{ fontWeight: "bold" }}>Bearer authentication</span> - token is added to bearer header
                value.
            </p>
            <div>
                <label>
                    <input type="checkbox" checked={enableBearerToken} onChange={bearerEnabledChanged} />
                    Enabled
                </label>
            </div>
            <input type="text" value={bearerToken} onChange={bearerChanged} disabled={!enableBearerToken} />
            <hr />

            <div className="box-line">
                <input type="button" value="Save" onClick={saveAndCheck} disabled={!isCancelEnabled()} />
                <input type="button" value="Cancel" onClick={cancelChanges} disabled={!isCancelEnabled()} />
                <input type="button" value="Reset to default" onClick={resetToDefault} />
            </div>
        </div>
    );
};

export default BackendSettingsPage;
