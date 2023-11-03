use erc20_payment_lib::err_custom_create;
use erc20_payment_lib::error::PaymentError;
use lazy_static::lazy_static;
use regex::Regex;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::TokioAsyncResolver;
use url::Url;
use web3::types::H160;

const DEFAULT_FAUCET_SRV_PREFIX: &str = "_eth-faucet._tcp";
const DEFAULT_ETH_FAUCET_HOST: &str = "faucet.testnet.golem.network";
const DEFAULT_LOOKUP_DOMAIN: &str = "dev.golem.network";

/// Resolves prefixes in the `DEFAULT_LOOKUP_DOMAIN`, see also `resolve_record`
pub async fn resolve_yagna_srv_record(prefix: &str) -> std::io::Result<String> {
    resolve_srv_record(&format!(
        "{}.{}",
        prefix.trim_end_matches('.'),
        DEFAULT_LOOKUP_DOMAIN
    ))
    .await
}

async fn resolve_faucet_url(chain_id: u64) -> Result<String, PaymentError> {
    let faucet_host = resolve_yagna_srv_record(DEFAULT_FAUCET_SRV_PREFIX)
        .await
        .unwrap_or_else(|_| DEFAULT_ETH_FAUCET_HOST.to_string());

    let port = match chain_id {
        80001 => 4002,
        5 => 4001,
        _ => return Err(err_custom_create!("Unsupported chain id: {}", chain_id)),
    };

    Ok(format!("http://{faucet_host}:{port}/donate"))
}

pub async fn resolve_srv_record(record: &str) -> std::io::Result<String> {
    let resolver: TokioAsyncResolver =
        TokioAsyncResolver::tokio(ResolverConfig::google(), ResolverOpts::default())?;
    let lookup = resolver.srv_lookup(record).await?;
    let srv = lookup
        .iter()
        .next()
        .ok_or_else(|| IoError::from(IoErrorKind::NotFound))?;
    let addr = format!(
        "{}:{}",
        srv.target().to_string().trim_end_matches('.'),
        srv.port()
    );

    log::debug!("Resolved address: {}", addr);
    Ok(addr)
}

/// Replace domain name in URL with resolved IP address
/// Hack required on windows to bypass failing resolution on Windows 10
/// Not needed when https://github.com/actix/actix-web/issues/1047 is resolved
pub async fn resolve_dns_record(request_url: &str) -> anyhow::Result<String> {
    let request_host = Url::parse(request_url)?
        .host()
        .ok_or_else(|| anyhow::anyhow!("Invalid url: {}", request_url))?
        .to_string();

    let address = resolve_dns_record_host(&request_host).await?;
    Ok(request_url.replace(&request_host, &address))
}

pub async fn resolve_dns_record_host(host: &str) -> anyhow::Result<String> {
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::google(), ResolverOpts::default())?;

    let response = resolver.lookup_ip(host).await?;
    let address = response
        .iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("DNS resolution failed for host: {}", host))?
        .to_string();
    Ok(address)
}

/// Try resolving hostname with `resolve_dns_record` or `resolve_dns_record_host`.
/// Returns the original URL if it fails.
pub async fn try_resolve_dns_record(request_url_or_host: &str) -> String {
    lazy_static! {
        static ref SCHEME_RE: Regex = Regex::new("(?i)^[a-z0-9\\-\\.]+?:").unwrap();
    }
    match {
        if SCHEME_RE.is_match(request_url_or_host) {
            resolve_dns_record(request_url_or_host).await
        } else {
            resolve_dns_record_host(request_url_or_host).await
        }
    } {
        Ok(url) => url,
        Err(e) => {
            log::warn!(
                "Error resolving hostname: {} url={}",
                e,
                request_url_or_host
            );
            request_url_or_host.to_owned()
        }
    }
}

pub async fn faucet_donate(chain_id: u64, address: H160) -> Result<(), PaymentError> {
    // TODO: Reduce timeout to 20-30 seconds when transfer is used.
    let client = awc::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .finish();

    let faucet_url = resolve_faucet_url(chain_id).await?;
    let request_url = format!("{}/0x{:x}", faucet_url, address);
    let request_url = try_resolve_dns_record(&request_url).await;
    log::debug!("Faucet request url: {}", request_url);
    let response = client
        .get(request_url)
        .send()
        .await
        .map_err(|e| err_custom_create!("Error getting response from faucet {}", e))?
        .body()
        .await
        .map_err(|e| err_custom_create!("Error getting payload from faucet {}", e))?;
    let response = String::from_utf8_lossy(response.as_ref());
    log::info!("Funds requested. Response = {}", response);
    // TODO: Verify tx hash
    Ok(())
}
