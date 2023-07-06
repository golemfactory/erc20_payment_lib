export function fromWei(wei: string | bigint): string {
    if (typeof wei === "string") {
        wei = BigInt(wei);
    }
    wei = wei / BigInt("1000000000000000000");
    return wei.toString();
}
