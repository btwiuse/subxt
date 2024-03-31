use clap::Parser;
use futures::StreamExt;
use kek::{snapshot, Chain};

#[derive(clap::Parser)]
pub struct Config {
    /// path to chain spec
    #[clap(short, long)]
    pub chain: String,
    /// storage key
    #[clap(short, long)]
    pub key: String,
    /// output file
    #[clap(short, long, default_value = "/dev/stdout")]
    pub output: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Config::parse();

    let api = Chain::parse(&app.chain).api().await?;

    // Use them!
    let mut sub = api
        .blocks()
        .subscribe_finalized()
        .await?
        .map(|block| (&app.chain, block));

    while let Some((chain, block)) = sub.next().await {
        let block = block?;
        eprintln!(
            "Chain {:?} hash={:?} number={:?}",
            chain,
            block.hash(),
            block.number()
        );
        snapshot(&api, &app.key, &app.output).await?;
        break;
    }

    Ok(())
}
