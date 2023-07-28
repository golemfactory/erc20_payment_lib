use bollard::container;
use bollard::container::StopContainerOptions;
use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::create_sqlite_connection;
use erc20_payment_lib::error::*;
use erc20_payment_lib::misc::{display_private_keys, load_private_keys};
use erc20_payment_lib::runtime::start_payment_engine;
use erc20_payment_lib::{config, err_custom_create};

use anyhow::{anyhow, bail};
use bollard::models::{PortBinding, PortMap};
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib_extra::{account_balance, AccountBalanceOptions};
use erc20_payment_lib_test::{GethContainer, SetupGethOptions};
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::join;
use tokio::time::Instant;

#[tokio::test(flavor = "multi_thread")]
async fn spawn_docker() -> Result<(), anyhow::Error> {
    let current = Instant::now();
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info,sqlx::query=warn,web3=warn".to_string()),
    );
    env_logger::init();

    let (_geth_container, conn) = match join!(
        GethContainer::create(SetupGethOptions::new()),
        create_sqlite_connection(None, None, true)
    ) {
        (Ok(geth_container), Ok(conn)) => (geth_container, conn),
        (Err(e), _) => bail!("Error when setup geth {}", e),
        (_, Err(e)) => bail!("Error when creating sqlite connections {}", e),
    };

    let mut config = config::Config::load("config-payments-local.toml")?;
    config.chain.get_mut("dev").unwrap().rpc_endpoints =
        vec!["http://127.0.0.1:8544/web3/dupa".to_string()];

    let (private_keys, _public_addrs) =
        load_private_keys("a8a2548c69a9d1eb7fdacb37ee64554a0896a6205d564508af00277247075e8f")?;
    display_private_keys(&private_keys);

    let add_opt = AdditionalOptions {
        keep_running: false,
        generate_tx_only: false,
        skip_multi_contract_check: false,
    };
    /*let _sp = start_payment_engine(
        &private_keys,
        &"db_test.sqlite",
        config.clone(),
        Some(conn.clone()),
        Some(add_opt),
    )
    .await?;*/

    let accounts_str = "0xB1C4D937A1b9bfC17a2Eb92D3577F8b66763bfC1,0x4799b810050f038288b4314501b70B1B9A49E1Dc,0xAc630277FB747Aa600d7A23EF08F5829861c639E,0xc48878a43476cd6cC5db772c492cB68D6d201249,0x0C5bE0eF7Fab4E847DD7bcc642a203220C730f21,0x1e97A59959394A7f3DFa753d1b8B12100b5d7Ce8,0x7754e3AE9A42D1Ad76afD691f1cFc7f0D4a82698,0x4caa30c14bC74bF3099CBe589a37DE53A4855EF6,0xEFac7290De2728630a4819C8443b4236a45B3e21,0x5774B9c27fAe1339386dED640fdc2717bCeD07C9,0x4E6076728Ba724Fc327B115ad3CEDB8aCbe37bd8,0x32Fc1A423F2B4aC21bD2679bD160e418598ACFC7,0xb33266F2A44209Fdb59bdc98feB6474DB1cF83E0,0x7FEDa0B256EB12FCFEec66f44F9e12CC631F0Df9,0xf77358be76125E0f95e206E24F1036C9F49D9692,0xff68350f138C4eB632beE2B59F640ab6d1e2e475,0xA9014205808373CeF5b6815f50e03842a99a9206,0x368E33F48F52755221B97389327B2eFf97c32700,0xa7ba45b534526513C0405e562cbbCDA50872a851,0x7bd3674a3212652D685488b6401Ef61452bEBB79,0xe4458E5080d9D8f39c235cc8B2090cDB02881925,0x4e94C42d9b7cBD4c8ae8254d0Cb2884e0a2055ac,0xEFa492B64cca91686Ed2FBbea29783C7b834CDDA,0x676e15C9375a925fbc1b0891f555D884788575cE,0xE6F185DAe234bC4369cFF548556A6E1Ce34A07E9,0xb9516A91e2a5F696430EEdc78d4F911f284DF35e,0x42a3906dEf13106ADCe76dC93405b354da3e2035,0xd4052DAbC05e0A4B04F493612af2e5D1055978ac,0x1eA5eeAD1Ba9CCD7A026f226c5e48e8781573562,0xbfb29b133aA51c4b45b49468F9a22958EAFeA6fa,0x653b48E1348F480149047AA3a58536eb0dbBB2E2,0x2E9e88A1f32Ea12bBaF3d3eb52a71c8224451431,0x40982A8F07A39DA509581751648efCadB276f4E9,0x9Ad40e3D756F59949485A280c572d8e715F14350,0x805D24c97d6dDFa63F402b8A5e16491229523a96,0x0E7E1c5aF8e3EA87527242a12C7A30e7E686090D,0x53fB152b2f69a48Bf1387f742e254725E5dB6b23,0x352734dAff396a59B56366b0A3C2A642B7643267,0x7372CAe62B3E5014dCC1060bA3741DeDBa28C7BB,0x6ae57Ecaeb101cc9CC0b9575CEC084B5cd39a8c6".to_string();
    let amounts = [
        "0.000001",
        "0.000002",
        "0.000004",
        "0.000008",
        "0.000016",
        "0.000032",
        "0.000064",
        "0.000128",
        "0.000256",
        "0.000512",
        "0.001024",
        "0.002048",
        "0.004096",
        "0.008192",
        "0.016384",
        "0.032768",
        "0.065536",
        "0.131072",
        "0.262144",
        "0.524288",
        "1.048576",
        "2.097152",
        "4.194304",
        "8.388608",
        "16.777216",
        "33.554432",
        "67.108864",
        "134.217728",
        "268.435456",
        "536.870912",
        "1073.741824",
        "2147.483648",
        "4294.967296",
        "8589.934592",
        "17179.869184",
        "34359.738368",
        "68719.476736",
        "137438.953472",
        "274877.906944",
        "549755.813888",
        "1099511.627776",
    ];

    let accounts_ref = accounts_str.to_lowercase();
    let accounts_map_ref = accounts_ref
        .split(',')
        .into_iter()
        .zip(amounts)
        .collect::<HashMap<&str, &str>>();

    let account_balance_options = AccountBalanceOptions {
        chain_name: "dev".to_string(),
        accounts: accounts_str,
        show_gas: true,
        show_token: true,
        block_number: None,
        tasks: 4,
        interval: Some(0.001),
    };

    let chain_cfg = config
        .chain
        .get(&account_balance_options.chain_name)
        .ok_or(err_custom_create!(
            "Chain {} not found in config file",
            account_balance_options.chain_name
        ))?;

    let payment_setup = PaymentSetup::new(&config, vec![], true, false, false, 1, 1, false)?;

    let web3 = payment_setup.get_provider(chain_cfg.chain_id)?;

    println!(
        "Connecting to geth... {:.2}s",
        current.elapsed().as_secs_f64()
    );
    while web3.eth().block_number().await.is_err() {
        tokio::time::sleep(Duration::from_secs_f64(0.04)).await;
    }
    println!(
        "Connected to geth after {:.2}s",
        current.elapsed().as_secs_f64()
    );

    let res = account_balance(account_balance_options.clone(), &config).await?;

    assert_eq!(res.iter().count(), 40);
    assert_eq!(accounts_map_ref.iter().count(), 40);

    for (key, val) in &res {
        if let Some(el) = accounts_map_ref.get(key.as_str()) {
            assert_eq!(val.gas_decimal.clone().unwrap(), *el);
        } else {
            bail!("Account {} not found in config file", key);
        }
    }

    //it's good idea to close sqlite connection before exit, thus we are sure that all transactions were written to db
    //TODO: wrap into RAII async drop hack
    conn.close().await;
    Ok(())
}
