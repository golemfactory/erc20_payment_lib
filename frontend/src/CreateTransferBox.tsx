import React, {useCallback, useContext} from "react";
import {ethers} from "ethers";
import {backendFetch} from "./common/BackendCall";
import {BackendSettingsContext} from "./BackendSettingsProvider";
import {useConfig} from "./ConfigProvider";


interface CreateTransferBoxProps {
    selectedChain: string | null;
    selectedAccount: string | null;
}

const CreateTransferBox = (props: CreateTransferBoxProps) => {
    const {backendSettings} = useContext(BackendSettingsContext);
    const config = useConfig();

    const [inputTo, setInputTo] = React.useState<string>("");
    const [inputAmount, setInputAmount] = React.useState<string>("");
    const [inputToValid, setInputToValid] = React.useState<boolean>(false);
    const [inputAmountValid, setInputAmountValid] = React.useState<boolean>(false);
    const [inputAmountBigInt, setInputAmountBigInt] = React.useState<bigint>(BigInt(0));
    const [inputUseGas, setInputUseGas] = React.useState<string>("token");

    const setInputToRandom = useCallback(() => {
        const bytes = ethers.utils.randomBytes(20);
        setInputTo(ethers.utils.getAddress(ethers.utils.hexlify(bytes)));
    }, []);

    const sendTransfer = useCallback(async () => {
        if (inputToValid && props.selectedChain) {

            const response = await backendFetch(backendSettings, `/transfers/new`, {
                method: "POST",
                body: JSON.stringify({
                    "from": props.selectedAccount,
                    "to": inputTo,
                    "amount": inputAmountBigInt.toString(),
                    "chain": parseInt(props.selectedChain),
                    "token": inputUseGas ? null : config.chainSetup[parseInt(props.selectedChain)].glmAddress,
                }),
            })
            const response_json = await response.text();
            console.log(response_json)
        }
    }, [props.selectedAccount, inputTo, inputToValid, props.selectedChain, inputAmountBigInt, inputUseGas, config]);


    return (
        <div className={"create-transfer-box"} style={{display: "flex", flexDirection: "column", padding: 20}}>
            <h4>
                Create transfer
            </h4>
            <div>
                from: {props.selectedAccount}
            </div>
            <div>
                <input type="text" placeholder="To (address)" onChange={e => setInputTo(e.target.value)}
                       value={inputTo}/>
                {inputToValid ? inputTo : "Invalid address"}
                <button onClick={e => setInputToRandom()}>Random</button>
            </div>
            <div>
                <input type="text" placeholder="Amount" onChange={e => setInputAmount(e.target.value)}
                       value={inputAmount}/>
                {inputAmountValid ? inputAmountBigInt.toString() : "Invalid amount"}
            </div>
            <div>
                <select onChange={e => setInputUseGas(e.target.value)}>
                    <option selected={inputUseGas == "gas"} value="gas">Gas</option>
                    <option selected={inputUseGas == "token"} value="token">GLM token
                        ({props.selectedChain ? config.chainSetup[parseInt(props.selectedChain)].glmAddress : ""})
                    </option>
                </select>
            </div>
            <div>
                <button onClick={e => sendTransfer()}>Send</button>
            </div>
        </div>
    );
}

export default CreateTransferBox;