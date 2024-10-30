use std::env;

use aggregator::Aggregator;
use eyre::Result;
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
    // Initialize tracing
    init_tracing();

    let args: Vec<String> = env::args().collect();
    match args.len() {
        2 => {
            // Correct number of arguments, continue with the private key
            let chain = args[1].clone().into();
            let mut aggregator = Aggregator::new(chain).await?;
            aggregator.run().await?
        }
        _ => {
            error!("Usage: {} <chain> ", args[0]);
            error!("Only the chain is expected as argument");
            std::process::exit(1);
        }
    }

    // Keep the main thread running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down");

    Ok(())
}
