#![allow(missing_docs)]
use clap::Parser;
use futures::StreamExt;
use subxt::{client::OnlineClient, lightclient::LightClient, PolkadotConfig};

#[derive(clap::Parser)]
pub struct Config {
    /// path to chain spec
    #[clap(short, long)]
    pub chain: String,
    /// storage key
    #[clap(short, long)]
    pub key: String,
    /// output file
    #[clap(short, long)]
    pub output: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Config::parse();

    // read chain spec as string
    let chain_spec = std::fs::read_to_string(&app.chain)?;

    // Instantiate a light client with the Polkadot relay chain
    let (_lightclient, polkadot_rpc) = LightClient::relay_chain(chain_spec)?;

    // Create Subxt clients from these Smoldot backed RPC clients.
    let polkadot_api = OnlineClient::<PolkadotConfig>::from_rpc_client(polkadot_rpc).await?;

    // Use them!
    let mut polkadot_sub = polkadot_api
        .blocks()
        .subscribe_finalized()
        .await?
        .map(|block| (basename(&app.chain), block));

    while let Some((chain, block)) = polkadot_sub.next().await {
        let block = block?;
        eprintln!(
            "Chain {:?} hash={:?} number={:?}",
            chain,
            block.hash(),
            block.number()
        );
        snapshot(&polkadot_api, &app.key, &app.output).await?;
        break;
    }

    Ok(())
}

async fn snapshot<T: subxt::Config>(
    polkadot_api: &OnlineClient<T>,
    key: &str,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut key = key.to_lowercase();
    if key.starts_with("0x") {
        key = key[2..].to_string();
    }
    let raw_key: Vec<u8> = hex::decode(&key)?;
    let storage = polkadot_api
        .storage()
        .at_latest()
        .await?
        .fetch_raw(raw_key)
        .await?;
    if let Some(snapshot) = storage {
        use std::io::Write;
        let mut file = std::fs::File::create(output)?;
        let encoded = hex::encode(&snapshot);
        file.write_all(encoded.as_bytes())?;
        eprintln!("{} bytes written to snapshot.hex", snapshot.len());
        Ok(())
    } else {
        Err(format!("No storage value found for key 0x{}", key).into())
    }
}

fn basename(path: &str) -> &str {
    std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
}
