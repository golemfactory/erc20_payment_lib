import React from "react";
import { DateTime, Interval } from "luxon";
import "./DateBox.css";

interface DateBoxProps {
    date: string | null | undefined;
    title: string;
    minimal: boolean | null;
}

const DateBox = (props: DateBoxProps) => {
    const dateNow = DateTime.now();
    const dateStr = props.date;

    const title = props.title;
    const date = "N/A";
    const msg = "-";
    let extraInfo = "Date not yet available";

    function render(title: string, date: string, msg: string, extraInfo: string) {
        if (props.minimal) {
            return (
                <div title={extraInfo} className={"date-container"}>
                    <div className={"date-container-date"}>{date}</div>
                </div>
            );
        }
        return (
            <div title={extraInfo} className={"date-container"}>
                <div className={"date-container-title"}>{title}</div>
                <div className={"date-container-date"}>{date}</div>
                <div className={"date-container-msg"}>{msg}</div>
            </div>
        );
    }

    if (dateStr == null) {
        return render(title, date, msg, extraInfo);
    }

    try {
        const luxonDate = DateTime.fromISO(dateStr);
        const currentDate = DateTime.now();

        const interval = Interval.fromDateTimes(luxonDate, dateNow);

        let message;
        if (interval.length("days") > 3) {
            message = Math.floor(interval.length("days")) + " days ago";
        } else if (interval.length("hours") > 3) {
            message = Math.floor(interval.length("hours")) + " hours ago";
        } else if (interval.length("minutes") > 3) {
            message = Math.floor(interval.length("minutes")) + " min. ago";
        } else {
            message = Math.floor(interval.length("seconds")) + " sec. ago";
        }

        let dateMsg = luxonDate.toFormat("yyyy-LL-dd HH:mm:ss");
        if (luxonDate.toFormat("yyyy-LL-dd") === currentDate.toFormat("yyyy-LL-dd")) {
            dateMsg = luxonDate.toFormat("HH:mm:ss");
        }

        extraInfo = "Iso date: " + luxonDate.toUTC().toFormat("yyyy-LL-dd HH:mm:ss");
        return render(title, dateMsg, message, extraInfo);
    } catch (e) {
        return render(title, "error", `${e}`, `${e}`);
    }
};

export default DateBox;
