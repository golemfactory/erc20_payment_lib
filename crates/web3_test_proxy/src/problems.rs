use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointSimulateProblems {
    pub timeout_chance: f64,
    pub min_timeout_ms: f64,
    pub max_timeout_ms: f64,
    pub error_chance: f64,
    pub malformed_response_chance: f64,
    pub skip_sending_raw_transaction_chance: f64,
    pub send_transaction_but_report_failure_chance: f64,
    pub allow_only_parsed_calls: bool,
    pub allow_only_single_calls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValuesChangeOptions {
    pub timeout_chance: Option<f64>,
    pub min_timeout_ms: Option<f64>,
    pub max_timeout_ms: Option<f64>,
    pub error_chance: Option<f64>,
    pub malformed_response_chance: Option<f64>,
    pub skip_sending_raw_transaction_chance: Option<f64>,
    pub send_transaction_but_report_failure_chance: Option<f64>,
    pub allow_only_parsed_calls: Option<bool>,
    pub allow_only_single_calls: Option<bool>,
}

impl Default for EndpointSimulateProblems {
    fn default() -> Self {
        Self {
            timeout_chance: 0.0,
            min_timeout_ms: 0.0,
            max_timeout_ms: 0.0,
            error_chance: 0.0,
            malformed_response_chance: 0.0,
            skip_sending_raw_transaction_chance: 0.0,
            send_transaction_but_report_failure_chance: 0.0,
            allow_only_parsed_calls: true,
            allow_only_single_calls: true,
        }
    }
}

impl EndpointSimulateProblems {
    pub fn apply_change(&mut self, change: &ValuesChangeOptions) {
        if let Some(timeout_chance) = change.timeout_chance {
            self.timeout_chance = timeout_chance;
        }
        if let Some(min_timeout_ms) = change.min_timeout_ms {
            self.min_timeout_ms = min_timeout_ms;
        }
        if let Some(max_timeout_ms) = change.max_timeout_ms {
            self.max_timeout_ms = max_timeout_ms;
        }
        if let Some(error_chance) = change.error_chance {
            self.error_chance = error_chance;
        }
        if let Some(malformed_response_chance) = change.malformed_response_chance {
            self.malformed_response_chance = malformed_response_chance;
        }
        if let Some(skip_sending_raw_transaction_chance) =
            change.skip_sending_raw_transaction_chance
        {
            self.skip_sending_raw_transaction_chance = skip_sending_raw_transaction_chance;
        }
        if let Some(send_transaction_but_report_failure_chance) =
            change.send_transaction_but_report_failure_chance
        {
            self.send_transaction_but_report_failure_chance =
                send_transaction_but_report_failure_chance;
        }
        if let Some(allow_only_parsed_calls) = change.allow_only_parsed_calls {
            self.allow_only_parsed_calls = allow_only_parsed_calls;
        }
        if let Some(allow_only_single_calls) = change.allow_only_single_calls {
            self.allow_only_single_calls = allow_only_single_calls;
        }
    }
}
