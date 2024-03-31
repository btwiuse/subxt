#![allow(missing_docs)]
use futures::StreamExt;
use subxt::{client::OnlineClient, lightclient::LightClient, PolkadotConfig};

const POLKADOT_SPEC: &str = include_str!("../../artifacts/demo_chain_specs/polkadot.json");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The lightclient logs are informative:
    tracing_subscriber::fmt::init();

    // Instantiate a light client with the Polkadot relay chain,
    // and connect it to Asset Hub, too.
    let (lightclient, polkadot_rpc) = LightClient::relay_chain(POLKADOT_SPEC)?;

    // Create Subxt clients from these Smoldot backed RPC clients.
    let polkadot_api = OnlineClient::<PolkadotConfig>::from_rpc_client(polkadot_rpc).await?;

    // Use them!
    let mut polkadot_sub = polkadot_api
        .blocks()
        .subscribe_finalized()
        .await?
        .map(|block| ("Polkadot", block));

    let key : Vec::<u8> = hex::decode("ede8e4fdc3c8b556f0ce2f77fc2575e396d38fd45bc038faa9586fa93aa03ef7")?;
    let storage = polkadot_api.storage().at_latest().await?.fetch_raw(key).await?;
    if let Some(snapshot) = storage {
        use std::io::Write;
        let mut file = std::fs::File::create("snapshot.hex")?;
        let encoded = hex::encode(&snapshot);
        file.write_all(encoded.as_bytes())?;
        println!("{} bytes written to snapshot.hex", snapshot.len());
        return Ok(());
    }
    dbg!(storage);

    while let Some((chain, block)) = polkadot_sub.next().await {
        let block = block?;
        println!("     Chain {:?} hash={:?} number={:?}", chain, block.hash(), block.number());
    }

    Ok(())
}
