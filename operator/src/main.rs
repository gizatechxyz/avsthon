use eyre::Result;
use operator::Operator;
use std::env;
use time::macros::format_description;
use tracing::error;
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

    let args: Vec<String> = env::args().collect();
    match args.len() {
        3 => {
            // Correct number of arguments, continue with the private key
            let private_key = args[1].clone();
            let chain = args[2].clone().into();
            let operator = Operator::new(&private_key, chain).await?;
            operator.run().await
        }
        _ => {
            error!("Usage: {} <private_key> <chain>", args[0]);
            error!("Both the private key and chain are expected as arguments");
            std::process::exit(1);
        }
    }
}
