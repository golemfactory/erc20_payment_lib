import React from "react";
import Web3Box from "./Web3Box";

const Web3Status = () => {
    return (
        <div>
            <h3>Web3 RPC endpoints info</h3>
            <Web3Box selectedChain={null} />
        </div>
    );
};

export default Web3Status;
