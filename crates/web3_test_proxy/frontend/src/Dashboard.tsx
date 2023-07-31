import React from "react";
import "./Dashboard.css";
import { Routes, Route, Link } from "react-router-dom";
import { useConfigResult } from "./ConfigProvider";
import BackendSettingsPage from "./BackendSettingsPage";
import Monitor from "./Monitor";
import SingleCallInfo from "./SingleCallInfo";
import Endpoints from "./Endpoints";
import WelcomePage from "./WelcomePage";

const Dashboard = () => {
    const configResult = useConfigResult();

    if (configResult.error) {
        return (
            <div>
                <div>{configResult.error}</div>
                <BackendSettingsPage />
            </div>
        );
    }
    if (configResult.config == null) {
        return <div>Loading... {configResult.progress}</div>;
    }
    return (
        <div>
            <div>
                <div className="top-header">
                    <div className="top-header-title">Web3 proxy panel</div>
                    <div className="top-header-navigation">
                        <Link to="/">Main</Link>
                        <Link to="/monitor">Monitor</Link>
                        <Link to="/endpoints">Endpoints</Link>
                        <Link to="/page3">Page 3</Link>
                    </div>
                </div>
                <div className="main-content">
                    <Routes>
                        <Route path="/" element={<WelcomePage />} />
                        <Route
                            path="monitor"
                            element={
                                <div>
                                    <Monitor />
                                </div>
                            }
                        />
                        <Route path="/call/:key/:callNo" element={<SingleCallInfo />} />

                        <Route
                            path="call"
                            element={
                                <div>
                                    <SingleCallInfo />
                                </div>
                            }
                        />
                        <Route
                            path="endpoints"
                            element={
                                <div>
                                    <Endpoints />
                                </div>
                            }
                        />
                        <Route
                            path="page3"
                            element={
                                <div>
                                    <BackendSettingsPage />
                                </div>
                            }
                        />
                    </Routes>
                </div>
            </div>
        </div>
    );
};

export default Dashboard;
