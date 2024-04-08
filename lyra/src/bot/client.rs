#[inline]
#[allow(dead_code)]
fn parse_directive(parsed: &str) -> tracing_subscriber::filter::Directive {
    parsed
        .parse()
        .unwrap_or_else(|e| panic!("invalid directive `{parsed}`: {e}"))
}

#[tracing::instrument(err)]
pub async fn run() -> Result<(), super::error::RunError> {
    color_eyre::install()?;
    dotenvy::dotenv()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
                .from_env_lossy(), // .add_directive(parse_directive("lyra=trace")),
                                   // .add_directive(parse_directive("lavalink_rs=trace")),
        )
        .init();

    Ok(super::runner::start().await?)
}
