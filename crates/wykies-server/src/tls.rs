#[cfg(not(feature = "disable-tls"))]
/// Patterned on example from https://github.com/actix/examples/tree/master/https-tls/rustls
#[tracing::instrument(ret, err(Debug))]
pub fn load_rustls_config() -> anyhow::Result<rustls::ServerConfig> {
    use anyhow::Context as _;

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed install crypto provider");

    // init server config builder with safe defaults
    let config = rustls::ServerConfig::builder().with_no_client_auth();

    // load TLS key/cert files
    let cert_path = std::path::PathBuf::from("cert.pem")
        .canonicalize()
        .context("failed to canonicalize path to cert.pem file")?;
    let key_path = std::path::PathBuf::from("key.pem")
        .canonicalize()
        .context("failed to canonicalize path to key.pem file")?;
    let cert_file = &mut std::io::BufReader::new(
        std::fs::File::open(&cert_path)
            .with_context(|| format!("failed to open cert file: {cert_path:?}"))?,
    );

    // convert files to key/cert objects
    let cert_chain = rustls_pki_types::pem::ReadIter::new(cert_file)
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("failed to extract certificate from: {cert_path:?}"))?;
    use rustls_pki_types::pem::PemObject;
    let key = rustls_pki_types::PrivateKeyDer::from_pem_file(&key_path)
        .with_context(|| format!("failed to extract keys from: {key_path:?}"))?;

    config
        .with_single_cert(cert_chain, key)
        .context("key_der invalid or private key does not match end-entity")
}
