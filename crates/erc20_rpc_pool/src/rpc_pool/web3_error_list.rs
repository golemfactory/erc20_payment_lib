pub fn check_if_proper_rpc_error(err: String) -> bool {
    if err.contains("transfer amount exceeds balance") {
        return true;
    }
    if err.contains("transfer amount exceeds balance") {
        return true;
    }
    false
}
