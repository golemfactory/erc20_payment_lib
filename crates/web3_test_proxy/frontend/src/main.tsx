import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import Dashboard from "./Dashboard";
import { ConfigProvider } from "./ConfigProvider";
import { BrowserRouter } from "react-router-dom";
import { Routes, Route } from "react-router-dom";
import { BackendSettingsProvider } from "./BackendSettingsProvider";

const rootEl = document.getElementById("root");
if (!rootEl) {
    throw new Error("No root element found");
}
const root = ReactDOM.createRoot(rootEl);
const baseUri = document.baseURI;
//check if string inside uri
if (FRONTEND_BASE != "/" && !baseUri.includes(FRONTEND_BASE)) {
    root.render(
        <div>
            <p>Invalid base URI, navigate to {FRONTEND_BASE}</p>
        </div>,
    );
} else {
    root.render(
        <React.StrictMode>
            <BackendSettingsProvider>
                <ConfigProvider>
                    <BrowserRouter basename={FRONTEND_BASE}>
                        <Routes>
                            <Route path="/*" element={<Dashboard />} />
                        </Routes>
                    </BrowserRouter>
                </ConfigProvider>
            </BackendSettingsProvider>
        </React.StrictMode>,
    );
}
