import React from "react";
import AllowanceBox from "./AllowanceBox";

const AllowanceBoxDesignTime = () => {
    const allowance1 = {
        allowance: "115792089237316195423570985008687907853269984665640564039457584007913129639935",
        chainId: 987789,
        confirmDate: "2022-12-23T13:37:47.827436700Z",
        error: null,
        feePaid: null,
        id: 9,
        owner: "0x001066290077e38f222cc6009c0c7a91d5192303",
        spender: "0xbcfe9736a4f5bf2e43620061ff3001ea0d003c0f",
        tokenAddr: "0xec9f23c207018a444f9351df3d7937f609870667",
        txId: null,
    };

    const allowance2 = {
        allowance: "12200000000000000000",
        chainId: 987789,
        confirmDate: null,
        error: null,
        feePaid: null,
        id: 10,
        owner: "0x101066290077e28f222cc6009c0c7a91d5192303",
        spender: "0xbcfe9736a4f5bf2e43620061ff3001ea0d003c0f",
        tokenAddr: "0xef9f23c207018a444f9351df3d7937f609870667",
        txId: null,
    };

    return (
        <div>
            <div className={"padding"}>
                <h1>Allowance box design time</h1>
                <AllowanceBox allowance={allowance1}></AllowanceBox>
                <AllowanceBox allowance={allowance2}></AllowanceBox>
            </div>
        </div>
    );
};

export default AllowanceBoxDesignTime;
