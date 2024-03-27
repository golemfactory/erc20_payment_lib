import React, { useCallback, useContext, useEffect } from "react";
import "./CreateTransferBox.css";

import { ethers } from "ethers";
import { backendFetch } from "./common/BackendCall";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { useConfig } from "./ConfigProvider";
import ContractDetails from "./ContractDetails";
import { DateTime } from "luxon";

interface CreateTransferBoxProps {
    selectedChain: string | null;
    selectedAccount: string | null;
}

function random_id(length: number) {
    let result = "";
    const start_characters = "abcdefghijklmnopqrstuvwxyz";
    const characters = "abcdefghijklmnopqrstuvwxyz0123456789";
    let counter = 0;
    while (counter < length) {
        if (counter == 0) {
            result += start_characters.charAt(Math.floor(Math.random() * start_characters.length));
        } else {
            result += characters.charAt(Math.floor(Math.random() * characters.length));
        }
        counter += 1;
    }
    return result;
}

const CreateTransferBox = (props: CreateTransferBoxProps) => {
    const { backendSettings } = useContext(BackendSettingsContext);
    const config = useConfig();

    const [inputTo, setInputTo] = React.useState<string>("");
    const [inputAmount, setInputAmount] = React.useState<string>("");
    const [inputToValid, setInputToValid] = React.useState<boolean>(false);
    const [inputAmountValid, setInputAmountValid] = React.useState<boolean>(false);
    const [inputAmountBigInt, setInputAmountBigInt] = React.useState<ethers.BigNumber>(ethers.BigNumber.from(0));
    const [inputUseGas, setInputUseGas] = React.useState<string>("token");
    const [inputClearData, setInputClearData] = React.useState<boolean>(true);
    const [isSending, setIsSending] = React.useState<boolean>(false);
    const [dueDateString, setDueDateString] = React.useState<string>("");
    const [paymentID, setPaymentID] = React.useState<string>("");

    const clearData = useCallback(() => {
        setInputTo("");
        setInputAmount("");
        setInputUseGas("token");
    }, []);

    const setInputToRandom = useCallback(() => {
        const bytes = ethers.utils.randomBytes(20);
        setInputTo(ethers.utils.getAddress(ethers.utils.hexlify(bytes)));
    }, []);

    useEffect(() => {
        setInputToValid(ethers.utils.isAddress(inputTo));
    }, [inputTo]);
    useEffect(() => {
        try {
            if (inputAmount == "") {
                setInputAmountValid(false);
                return;
            }
            const amount = ethers.utils.parseUnits(inputAmount, 18);
            setInputAmountValid(true);
            setInputAmountBigInt(amount);
        } catch (e) {
            setInputAmountValid(false);
        }
    }, [inputAmount]);

    const isTransferValid = useCallback(() => {
        return inputToValid && inputAmountValid;
    }, [inputToValid, inputAmountValid]);

    const sendTransfer = useCallback(async () => {
        if (inputToValid && props.selectedChain) {
            setIsSending(true);
            const response = await backendFetch(backendSettings, `/transfers/new`, {
                method: "POST",
                body: JSON.stringify({
                    from: props.selectedAccount,
                    to: inputTo,
                    amount: inputAmountBigInt.toString(),
                    chain: parseInt(props.selectedChain),
                    token: inputUseGas == "gas" ? null : config.chainSetup[parseInt(props.selectedChain)].glmAddress,
                    dueDate: dueDateString ? DateTime.fromISO(dueDateString).toISO() : null,
                    useInternal: false,
                    depositId: null,
                }),
            });
            //sleep for one seconds
            await new Promise((r) => setTimeout(r, 200));
            const response_json = await response.text();
            console.log(response_json);
            if (inputClearData) {
                clearData();
            }
            setPaymentID(random_id(10));

            setIsSending(false);
        }
    }, [
        props.selectedAccount,
        inputTo,
        inputToValid,
        props.selectedChain,
        inputAmountBigInt,
        inputUseGas,
        config,
        inputClearData,
    ]);

    if (props.selectedChain == null) {
        return <div>Chain not selected</div>;
    }
    return (
        <div className="create-transfer-box">
            <div className="create-transfer-box-header">Create transfer</div>
            <div>
                from:{" "}
                <ContractDetails
                    contractAddress={props.selectedAccount}
                    chainId={parseInt(props.selectedChain)}
                    isAddress={"Receiver id"}
                />
            </div>
            <div>
                <div className="create-transfer-box-label">
                    to:{" "}
                    {inputToValid ? (
                        <ContractDetails
                            contractAddress={inputTo}
                            chainId={parseInt(props.selectedChain)}
                            isAddress={true}
                        />
                    ) : (
                        "Invalid address"
                    )}
                </div>
                <div>
                    <input
                        className="create-transfer-box-address-input"
                        type="text"
                        placeholder="To (address)"
                        onChange={(e) => setInputTo(e.target.value)}
                        value={inputTo}
                    />
                    <button onClick={(_e) => setInputToRandom()}>Random</button>
                </div>
            </div>
            <div>
                <div className="create-transfer-box-label">
                    value:
                    {inputAmountValid ? ethers.utils.formatEther(inputAmountBigInt.toString()) : "Invalid amount "}
                    {inputUseGas == "token"
                        ? config.chainSetup[parseInt(props.selectedChain)].currencyGlmSymbol
                        : config.chainSetup[parseInt(props.selectedChain)].currencyGasSymbol}
                </div>
                <div>
                    <input
                        type="text"
                        placeholder="Amount"
                        onChange={(e) => setInputAmount(e.target.value)}
                        value={inputAmount}
                    />
                    <button onClick={(_e) => setInputAmount("0.000000000000000001")}>1 wei</button>
                    <button onClick={(_e) => setInputAmount("0.000000001")}>1 Gwei</button>
                    <button onClick={(_e) => setInputAmount("0.001")}>1 mETH</button>
                </div>
            </div>
            <div>
                <div className="create-transfer-box-label">
                    token:{" "}
                    {inputUseGas == "token" ? (
                        <ContractDetails
                            contractAddress={config.chainSetup[parseInt(props.selectedChain)].glmAddress}
                            chainId={parseInt(props.selectedChain)}
                            isAddress={true}
                        />
                    ) : (
                        "Native token"
                    )}
                </div>
                <div>
                    <select onChange={(e) => setInputUseGas(e.target.value)}>
                        <option selected={inputUseGas == "gas"} value="gas">
                            Native/gas token ({config.chainSetup[parseInt(props.selectedChain)].currencyGasSymbol})
                        </option>
                        <option selected={inputUseGas == "token"} value="token">
                            ERC20 token (
                            {props.selectedChain
                                ? config.chainSetup[parseInt(props.selectedChain)].currencyGlmSymbol
                                : ""}
                            )
                        </option>
                    </select>
                </div>
            </div>
            <div>
                <div className="create-transfer-box-label">
                    Due date: {dueDateString ? DateTime.fromISO(dueDateString).toISO() : "No due date"}
                </div>
                <div>
                    <input
                        className="create-transfer-box-due-date-input"
                        type="text"
                        placeholder="Due date"
                        onChange={(e) => setDueDateString(e.target.value)}
                        value={dueDateString}
                    ></input>
                    <button onClick={(_e) => setDueDateString(DateTime.now().toISO() ?? "")}>Current</button>
                    <button onClick={(_e) => setDueDateString(DateTime.now().plus({ minute: 1 }).toISO() ?? "")}>
                        curr. +1 min
                    </button>
                    <button onClick={(_e) => setDueDateString(DateTime.now().plus({ minute: 5 }).toISO() ?? "")}>
                        curr. +5 min
                    </button>
                </div>
            </div>
            <div>
                <div className="create-transfer-box-label">Payment id (should be unique): {paymentID}</div>
                <div>
                    <input
                        className="create-uuid-box-uuid-input"
                        type="text"
                        placeholder="Payment id"
                        onChange={(e) => setPaymentID(e.target.value)}
                        value={paymentID}
                    ></input>
                    <button onClick={(_e) => setPaymentID(random_id(10))}>Random</button>
                </div>
            </div>
            <div>
                <input
                    id="cbClearData"
                    type="checkbox"
                    checked={inputClearData}
                    onChange={(_e) => setInputClearData(!inputClearData)}
                />
                <label htmlFor="cbClearData">Clear data after send</label>
            </div>
            <div>
                <button disabled={isSending || !isTransferValid()} onClick={(_e) => sendTransfer()}>
                    Send
                </button>
            </div>
        </div>
    );
};

export default CreateTransferBox;
