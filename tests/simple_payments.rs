use bollard::container;
use futures_util::TryStreamExt;

pub struct ImageName {
    pub user: Option<String>,
    pub repository: String,
    pub tag: String,
}

impl ImageName {
    pub fn to_base_name(&self) -> String {
        let mut res = String::new();
        if let Some(user) = &self.user {
            res.push_str(user);
            res.push('/');
        }
        res.push_str(&self.repository);
        res
    }

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

#[tokio::test]
async fn spawn_docker() -> Result<(), anyhow::Error> {
    use bollard::{image, service::HostConfig, Docker};
    let image_name = "geth".to_string();
    println!("Building image: {}", image_name);
    let docker = match Docker::connect_with_local_defaults() {
        Ok(docker) => docker,
        Err(err) => {
            log::error!("Failed to connect to docker: {}", err);
            return Err(anyhow::anyhow!("Failed to connect to docker: {}", err));
        }
    };
    match docker.version().await {
        Ok(version) => {
            println!(
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
        println!("Image found {}", image_name);
        image
    } else {
        println!("Image not found, downloading: {}", image_name);
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

    println!("Image id extracted {}", image_id);

    let env_opt = vec![
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
        "FAUCET_ACCOUNT_PRIVATE_KEY=078d8f6c16446cdb8efbee80535ce8cb32d5b69563bca33e5e6bc0f13f0666b3".to_string()];
    let container = docker
        .create_container::<String, String>(
            None,
            container::Config {
                image: Some(image_id.clone()),
                host_config: Some(HostConfig {
                    auto_remove: Some(true),
                    ..Default::default()
                }),
                env: Some(env_opt),
                cmd: Some(vec![
                    "python".to_string(),
                    "-u".to_string(),
                    "setup_chain.py".to_string(),
                ]),

                ..Default::default()
            },
        )
        .await?;
    let container_id = container.id;

    println!(" -- Container id: {}", &container_id[0..12]);

    docker
        .start_container::<String>(&container_id, None)
        .await?;

    Ok(())
}
