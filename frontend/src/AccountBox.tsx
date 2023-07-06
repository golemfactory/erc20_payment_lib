import React, { useCallback, useContext } from "react";
import AllowanceBox from "./AllowanceBox";
import Allowance from "./model/Allowance";
import AccountDetails from "./model/AccountDetails";
import { BackendSettingsContext } from "./BackendSettingsProvider";
import { backendFetch } from "./common/BackendCall";

interface AccountBoxProps {
    account: string | null;
}

const AccountBox = (props: AccountBoxProps) => {
    const [account, setAccount] = React.useState<AccountDetails | null>(null);
    const { backendSettings } = useContext(BackendSettingsContext);

    const loadAccountDetails = useCallback(async () => {
        const response = await backendFetch(backendSettings, `/account/${props.account}`);
        const response_json = await response.json();
        setAccount(response_json);
    }, [props.account]);

    React.useEffect(() => {
        loadAccountDetails().then();
    }, [loadAccountDetails]);

    function allowanceRow(allowance: Allowance, idx: number) {
        return (
            <div key={idx}>
                <AllowanceBox allowance={allowance} />
            </div>
        );
    }
    if (account === null) {
        return <div>Loading...</div>;
    }

    return (
        <div className={"account-box"}>
            <div className={"account-box-header"}>Account {props.account}</div>
            <div className={"account-box-body"}>
                <div>{JSON.stringify(account)}</div>
                <div>{account.allowances.map(allowanceRow)}</div>
            </div>
        </div>
    );
};

export default AccountBox;
