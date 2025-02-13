use anyhow::{bail, Context};
use rustls::{pki_types::PrivateKeyDer, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{fs::File, io::BufReader, path::PathBuf};
use tracing::instrument;

/// Patterned on example from https://github.com/actix/examples/tree/master/https-tls/rustls
#[instrument(ret, err(Debug))]
pub fn load_rustls_config() -> anyhow::Result<rustls::ServerConfig> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed install crypto provider");

    // init server config builder with safe defaults
    let config = ServerConfig::builder().with_no_client_auth();

    // load TLS key/cert files
    let cert_path = PathBuf::from("cert.pem")
        .canonicalize()
        .context("failed to canonicalize path to cert.pem file")?;
    let key_path = PathBuf::from("key.pem")
        .canonicalize()
        .context("failed to canonicalize path to key.pem file")?;
    let cert_file = &mut BufReader::new(
        File::open(&cert_path)
            .with_context(|| format!("failed to open cert file: {cert_path:?}"))?,
    );
    let key_file = &mut BufReader::new(
        File::open(&key_path).with_context(|| format!("failed to open key file: {key_path:?}"))?,
    );

    // convert files to key/cert objects
    let cert_chain = certs(cert_file)
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("failed to extract certificate from: {cert_path:?}"))?;
    let mut keys = pkcs8_private_keys(key_file)
        .map(|key| key.map(PrivateKeyDer::Pkcs8))
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("failed to extract keys from: {key_path:?}"))?;

    // exit if no keys could be parsed
    if keys.is_empty() {
        bail!("Could not locate PKCS 8 private keys.");
    }

    config
        .with_single_cert(cert_chain, keys.remove(0))
        .context("key_der invalid or private key does not match end-entity")
}
