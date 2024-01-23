import React, { useCallback, useContext } from "react";
import "./Web3Box.css";
import { backendFetch } from "./common/BackendCall";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import DateBox from "./DateBox";

interface Web3BoxProps {
    selectedChain: string | null;
}

interface Web3VerifyEndpointStatus {
    headSecondsBehind: number;
    checkTimeMs: number;
}
interface Web3VerifyEndpointResult {
    ok?: Web3VerifyEndpointStatus;
    noBlockInfo?: any;
    wrongChainId?: any;
    rpcWeb3Error?: string;
    otherNetworkError?: string;
    headBehind?: string;
    Unreachable?: string;
}

interface Web3RpcInfo {
    bonusFromLastChosen: number;
    endpointConsecutiveErrors: number;
    isAllowed: boolean;
    lastChosen: string | null;
    lastVerified: string | null;
    penaltyFromErrors: number;
    penaltyFromHeadBehind: number;
    penaltyFromLastCriticalError: number;
    penaltyFromMs: number;
    removedDate: string | null;
    verifyResult?: Web3VerifyEndpointResult;
}

interface Web3EndpointParams {
    backupLevel: number;
    maxHeadBehindSecs: number;
    maxNumberOfConsecutiveErrors: number;
    maxResponseTimeMs: number;
    minIntervalRequestMs: number;
    skipValidation: boolean;
    verifyIntervalSecs: number;
}

interface Web3RpcParams {
    chainId: number;
    endpoint: string;
    name: string;
    sourceId: string;
    web3EndpointParams: Web3EndpointParams;
}

interface RpcPoolEndpoint {
    web3RpcInfo: Web3RpcInfo;
    web3RpcParams: Web3RpcParams;
}

interface RpcPoolNetwork {
    chainId: number;
    chainNetwork: string;
    endpoints: [RpcPoolEndpoint];
}

interface RpcPool {
    networks: [RpcPoolNetwork];
}

interface Web3EndpointBoxProps {
    endpoint: RpcPoolEndpoint;
}

const Web3EndpointBox = (props: Web3EndpointBoxProps) => {
    return (
        <div
            className={
                "web3-endpoint-box " +
                (props.endpoint.web3RpcInfo.isAllowed
                    ? "web3-endpoint-box-allowed "
                    : "web3-endpoint-box-not-allowed ") +
                (props.endpoint.web3RpcInfo.lastVerified
                    ? "web3-endpoint-box-verified"
                    : "web3-endpoint-box-not-verified")
            }
        >
            <div>{props.endpoint.web3RpcParams.name}</div>
            <div>{props.endpoint.web3RpcParams.endpoint}</div>
            <div>{props.endpoint.web3RpcParams.sourceId}</div>
            <div style={{ display: "flex" }}>
                <div style={{ display: "flex" }}>
                    <DateBox date={props.endpoint.web3RpcInfo.lastChosen} title={"Last chosen"} />
                </div>
                <div className="web3-endpoint-box-last-verified">
                    <DateBox date={props.endpoint.web3RpcInfo.lastVerified} title={"Last verified"} />
                </div>
            </div>
            <div>{JSON.stringify(props.endpoint.web3RpcInfo.verifyResult)}</div>
        </div>
    );
};

const Web3Box = (_props: Web3BoxProps) => {
    //const config = useConfig();
    const { backendSettings } = useContext(BackendSettingsContext);
    const [rpcPool, setRpcPool] = React.useState<RpcPool | null>(null);

    const loadWeb3Data = useCallback(async () => {
        const response = await backendFetch(backendSettings, "/rpc_pool");
        const response_json = await response.json();
        setRpcPool(response_json);
    }, []);

    React.useEffect(() => {
        loadWeb3Data().then();
    }, [loadWeb3Data]);

    return (
        <>
            {rpcPool?.networks.map((network) => (
                <div key={network.chainId}>
                    <div className="web3-endpoint-box-header">{network.chainNetwork}</div>
                    <div className="web3-endpoint-box-container">
                        {network.endpoints.map((endpoint) => (
                            <Web3EndpointBox key={endpoint.web3RpcParams.endpoint} endpoint={endpoint} />
                        ))}
                    </div>
                </div>
            ))}
        </>
    );
};

export default Web3Box;
