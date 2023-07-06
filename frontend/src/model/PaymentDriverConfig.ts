import ChainSetup from "./ChainSetup";

interface PaymentDriverConfig {
    id: number;

    chainSetup: { [key: number]: ChainSetup };
}
export default PaymentDriverConfig;
