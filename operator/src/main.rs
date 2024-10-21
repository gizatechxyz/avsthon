use eyre::Result;
use operator::Operator;
use time::macros::format_description;
use tracing_subscriber::fmt;

fn init_tracing() {
    // Initialize the tracing subscriber with custom filter and format
    let format = fmt::format()
        .with_level(true)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_timer(fmt::time::UtcTime::new(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        )))
        .compact()
        .with_source_location(false)
        .with_ansi(true);

    tracing_subscriber::fmt()
        .event_format(format)
        .with_max_level(tracing::Level::INFO)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let operator = Operator::new().await?;
    operator.run().await
}
