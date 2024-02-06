use anyhow::anyhow;
use bollard::container::StopContainerOptions;
use bollard::models::{PortBinding, PortMap};
use bollard::{container, image, service::HostConfig, Docker};
use erc20_payment_lib_common::utils::*;
use futures_util::TryStreamExt;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::runtime::Handle;

pub struct ImageName {
    pub user: Option<String>,
    pub repository: String,
    pub tag: String,
}

impl ImageName {
    #[allow(dead_code)]
    pub fn to_base_name(&self) -> String {
        let mut res = String::new();
        if let Some(user) = &self.user {
            res.push_str(user);
            res.push('/');
        }
        res.push_str(&self.repository);
        res
    }

    #[allow(dead_code)]
    pub fn to_normalized_name(&self) -> String {
        let mut res = String::new();
        if let Some(user) = &self.user {
            res.push_str(user);
            res.push('/');
        }
        res.push_str(&self.repository);
        res.push(':');
        res.push_str(&self.tag);
        res
    }

    pub fn from_str_name(image_name: &str) -> anyhow::Result<Self> {
        let mut contains_alpha = false;
        for (pos, c) in image_name.chars().enumerate() {
            if c.is_whitespace() {
                return Err(anyhow::anyhow!(
                    "Invalid image name: {}. Cannot contain whitespaces",
                    image_name
                ));
            }
            if c.is_ascii_alphanumeric() {
                contains_alpha = true;
            } else if c == '-' || c == '_' || c == '.' || c == '/' || c == ':' {
                // ok
            } else {
                return Err(anyhow::anyhow!("Invalid image name: {}. Contains at least one invalid character: '{}' at pos {}", image_name, c, pos));
            }
        }
        if !contains_alpha {
            return Err(anyhow::anyhow!(
                "Invalid image name: {}. Must contain alphanumeric characters",
                image_name
            ));
        }

        if image_name.starts_with(':') {
            return Err(anyhow::anyhow!(
                "Invalid image name: {}. Cannot start with ':'",
                image_name
            ));
        }
        if image_name.starts_with('/') {
            return Err(anyhow::anyhow!(
                "Invalid image name: {}. Cannot start with '/'",
                image_name
            ));
        }
        if image_name.starts_with('/') {
            return Err(anyhow::anyhow!(
                "Invalid image name: {}. Cannot start with '/'",
                image_name
            ));
        }
        if image_name.matches(':').count() > 1 {
            return Err(anyhow::anyhow!(
                "Invalid image name: {}. ':' can occur only once",
                image_name
            ));
        }
        if image_name.matches('/').count() > 1 {
            return Err(anyhow::anyhow!(
                "Invalid image name: {}. '/' can occur only once",
                image_name
            ));
        }
        let (base_part, tag_part) = if image_name.contains(':') {
            let mut split = image_name.split(':');
            (
                split.next().expect("Split has to be here"),
                split.next().expect("Split has to be here"),
            )
        } else {
            (image_name, "latest")
        };
        if tag_part.is_empty() {
            return Err(anyhow::anyhow!(
                "Invalid image name: {}. Tag part cannot be empty",
                image_name
            ));
        }
        let (user, repo) = if base_part.contains('/') {
            let mut split = base_part.split('/');
            (
                Some(split.next().expect("Split has to be here")),
                split.next().expect("Split has to be here"),
            )
        } else {
            (None, base_part)
        };

        Ok(Self {
            user: user.map(|s| s.to_string()),
            repository: repo.to_string(),
            tag: tag_part.to_string(),
        })
    }
}

lazy_static! {
    static ref USED_PORTS: tokio::sync::Mutex<std::collections::HashSet<u16>> =
        tokio::sync::Mutex::new(std::collections::HashSet::new());
}

/// Returns available port pair, this is not PRODUCTION code, only for tests
/// DO NOT USE FOR PRODUCTION. it is not guaranteed to always work
async fn get_available_port_pair() -> Result<(u16, u16), anyhow::Error> {
    let port1 = 8544;
    let port2 = 8545;

    for _i in 0..100 {
        let random_skew = rand::random::<u16>() % 1000 * 2;
        if USED_PORTS.lock().await.contains(&(port1 + random_skew)) {
            continue;
        }
        if USED_PORTS.lock().await.contains(&(port2 + random_skew)) {
            continue;
        }
        if let (true, true) = tokio::join!(
            port_is_available(port1 + random_skew),
            port_is_available(port2 + random_skew)
        ) {
            USED_PORTS.lock().await.insert(port1 + random_skew);
            USED_PORTS.lock().await.insert(port1 + random_skew);
            return Ok((port1 + random_skew, port2 + random_skew));
        }
    }
    Err(anyhow!("Cannot find available port pair"))
}

async fn port_is_available(port: u16) -> bool {
    tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .is_ok()
}

pub struct SetupGethOptions {
    pub image_name: String,
    pub web3_port: Option<u16>,
    pub web3_proxy_port: Option<u16>,
    pub max_docker_lifetime: Duration,
}

impl SetupGethOptions {
    pub fn new() -> Self {
        Self {
            image_name: "scx1332/geth:lean".to_string(),
            web3_port: None,
            web3_proxy_port: None,
            max_docker_lifetime: Duration::from_secs_f64(60.0),
        }
    }

    pub fn max_docker_lifetime(mut self, max_docker_lifetime: Duration) -> Self {
        self.max_docker_lifetime = max_docker_lifetime;
        self
    }

    pub fn web3_port(mut self, web3_port: u16) -> Self {
        self.web3_port = Some(web3_port);
        self
    }

    pub fn web3_proxy_port(mut self, web3_proxy_port: u16) -> Self {
        self.web3_proxy_port = Some(web3_proxy_port);
        self
    }
}

impl Default for SetupGethOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GethContainer {
    pub docker: Docker,
    pub container_id: String,
    pub container_stopped: bool,
    pub web3_rpc_port: u16,
    pub web3_proxy_port: u16,
}

impl Drop for GethContainer {
    fn drop(&mut self) {
        let docker = self.docker.clone();
        let container_id = self.container_id.clone();

        if get_env_bool_value("ERC20_TEST_KEEP_DOCKER_CONTAINER") {
            return;
        }

        // This is async drop - probably good but not sure, need further investigation
        // it work only if multithreaded runtime is used
        if !self.container_stopped {
            tokio::task::block_in_place(move || {
                Handle::current().block_on(async move {
                    docker
                        .stop_container(container_id.as_str(), Some(StopContainerOptions { t: 0 }))
                        .await
                        .unwrap();
                });
            });
        }
    }
}
impl GethContainer {
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        self.docker
            .stop_container(&self.container_id, Some(StopContainerOptions { t: 0 }))
            .await?;
        self.container_stopped = true;
        Ok(())
    }

    pub async fn create(opt: SetupGethOptions) -> anyhow::Result<GethContainer> {
        let current = Instant::now();

        let image_name = "scx1332/geth:lean".to_string();
        log::debug!("Building image: {}", image_name);
        let docker = match Docker::connect_with_local_defaults() {
            Ok(docker) => docker,
            Err(err) => {
                log::error!("Failed to connect to docker: {}", err);
                return Err(anyhow::anyhow!("Failed to connect to docker: {}", err));
            }
        };
        match docker.version().await {
            Ok(version) => {
                log::debug!(
                    " -- connected to docker engine platform: {} version: {}",
                    version.platform.map(|pv| pv.name).unwrap_or("".to_string()),
                    version.version.unwrap_or_default()
                );
            }
            Err(err) => {
                log::error!("Failed to get docker service version: {}", err);
                return Err(anyhow::anyhow!(
                    "Cannot connect to docker engine, please check if docker is running"
                ));
            }
        };
        let parsed_name = ImageName::from_str_name(&image_name)?;

        let tag_from_image_name = parsed_name.tag;

        let image = docker.inspect_image(&image_name).await;
        //let image_id = image.id.unwrap();
        let image = if let Ok(image) = image {
            log::debug!("Image found {}", image_name);
            image
        } else {
            log::info!(
                "Image not found, downloading (it may take a while): {}",
                image_name
            );
            match docker
                .create_image(
                    Some(image::CreateImageOptions {
                        from_image: image_name.as_str(),
                        tag: &tag_from_image_name,
                        ..Default::default()
                    }),
                    None,
                    None,
                )
                .try_for_each(|_ev| async { Ok(()) })
                .await
            {
                Ok(_) => docker.inspect_image(&image_name).await?,
                Err(err) => {
                    log::error!("Failed to create image: {}", err);
                    return Err(anyhow::anyhow!("Failed to create image: {}", err));
                }
            }
        };
        let image_id = image.id.unwrap();
        let image_id = if image_id.starts_with("sha256:") {
            image_id.replace("sha256:", "")
        } else {
            log::error!("Image id is not sha256: {}", image_id);
            return Err(anyhow::anyhow!("Image id is not sha256: {}", image_id));
        };

        log::debug!("Image id extracted {}", image_id);

        let max_docker_lifetime = if get_env_bool_value("ERC20_TEST_KEEP_DOCKER_CONTAINER") {
            30 * 24 * 3600
        } else {
            opt.max_docker_lifetime.as_secs()
        };

        let env_opt = vec![
            format!("GETH_MAX_LIFESPAN={}", max_docker_lifetime),
            "CHAIN_ID=987789".to_string(),
            "CHAIN_NAME=GolemTestChain".to_string(),
            "CHAIN_TYPE=local".to_string(),
            "KEEP_RUNNING=1".to_string(),
            "PERIOD_IN_SECONDS_INT=2".to_string(),
            "SIGNER_ACCOUNT_PRIVATE_KEY=2f196e2e9ff66b9bd372ecbe0368c159d1e6a4c1f36c4222902fa345af35ddfb".to_string(),
            "SIGNER_ACCOUNT_PUBLIC_ADDRESS=0xa9932dA914AcDd62649081C599b0746CAb750c22".to_string(),
            "SIGNER_ACCOUNT_KEYSTORE_PASSWORD=d2fUH5loMsMXOkmdWAUO".to_string(),
            "MAIN_ACCOUNT_PRIVATE_KEY=a8a2548c69a9d1eb7fdacb37ee64554a0896a6205d564508af00277247075e8f".to_string(),
            "DISTRIBUTE_CONTRACT_ADDRESS=fill_me".to_string(),
            "MULTI_PAYMENT_CONTRACT_ADDRESS=0xF9861F83766CD507E0d2749B60d4fD6C68E5B96C".to_string(),
            "GLM_CONTRACT_ADDRESS=0xfff17584d526aba263025eE7fEF517E4A31D4246".to_string(),
            "FAUCET_ACCOUNT_PUBLIC_ADDRESS=0xafca53fc9628F0E7603bb2bf8E75F07Ee6442cE6".to_string(),
            "MAIN_ACCOUNT_PUBLIC_ADDRESS=0x4D6947E072C1Ac37B64600B885772Bd3f27D3E91".to_string(),
            "FAUCET_ACCOUNT_PRIVATE_KEY=078d8f6c16446cdb8efbee80535ce8cb32d5b69563bca33e5e6bc0f13f0666b3".to_string(),
        ];

        let (web3_proxy_port, geth_rpc_port) = if let (Some(web3_proxy_port), Some(geth_rpc_port)) =
            (opt.web3_proxy_port, opt.web3_port)
        {
            (web3_proxy_port, geth_rpc_port)
        } else {
            get_available_port_pair().await?
        };
        //let web3_proxy_port_str = format!("{web3_proxy_port}/tcp");
        //let geth_rpc_port_str = format!("{geth_rpc_port}/tcp");

        let mut port_mapping = PortMap::new();
        port_mapping.insert(
            "8545/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(geth_rpc_port.to_string()),
            }]),
        );
        port_mapping.insert(
            "8544/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(web3_proxy_port.to_string()),
            }]),
        );
        let mut exposed_ports = HashMap::new();
        exposed_ports.insert("8544/tcp".to_string(), HashMap::<(), ()>::new());
        exposed_ports.insert("8545/tcp".to_string(), HashMap::<(), ()>::new());

        let container = docker
            .create_container::<String, String>(
                None,
                container::Config {
                    image: Some(image_id.clone()),
                    host_config: Some(HostConfig {
                        auto_remove: Some(true),
                        port_bindings: Some(port_mapping),
                        ..Default::default()
                    }),
                    exposed_ports: Some(exposed_ports),
                    env: Some(env_opt),
                    cmd: None,

                    ..Default::default()
                },
            )
            .await?;
        let container_id = container.id;

        log::debug!(" -- Container id: {}", &container_id[0..12]);

        docker
            .start_container::<String>(&container_id, None)
            .await?;

        log::info!(
            "Container from image {} started in {:.2}s, web3_proxy port: {}, geth rpc port: {}",
            image_name,
            current.elapsed().as_secs_f64(),
            web3_proxy_port,
            geth_rpc_port
        );

        log::debug!(
            "Connecting to geth... {:.2}s",
            current.elapsed().as_secs_f64()
        );
        let web3 = web3::Web3::new(web3::transports::Http::new(&format!(
            "http://127.0.0.1:{}",
            geth_rpc_port
        ))?);
        while web3.eth().block_number().await.is_err() {
            tokio::time::sleep(Duration::from_secs_f64(0.04)).await;
        }
        log::info!(
            "Blockchain RPC ready in {:.2}s",
            current.elapsed().as_secs_f64()
        );

        Ok(GethContainer {
            docker,
            container_id,
            container_stopped: false,
            web3_rpc_port: geth_rpc_port,
            web3_proxy_port,
        })
    }
}
