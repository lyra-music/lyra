#[tracing::instrument(err)]
pub async fn run() -> Result<(), super::error::Run> {
    color_eyre::install()?;
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(crate::error::InstallDefaultCryptoProvider)?;

    Ok(super::runner::start().await?)
}
