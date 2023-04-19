// Copyright 2019-2023 Parity Technologies (UK) Ltd.
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

//! To run this example, a local polkadot node should be running. Example verified against polkadot v0.9.28-9ffe6e9e3da.
//!
//! E.g.
//! ```bash
//! curl "https://github.com/paritytech/polkadot/releases/download/v0.9.28/polkadot" --output /usr/local/bin/polkadot --location
//! polkadot --dev --tmp
//! ```

use futures::StreamExt;
use sp_keyring::AccountKeyring;
use std::time::Duration;
use subxt::blocks::ExtrinsicError;
use subxt::{tx::PairSigner, OnlineClient, PolkadotConfig};

#[subxt::subxt(runtime_metadata_path = "../artifacts/polkadot_metadata.scale")]
pub mod polkadot {}

/// Subscribe to all events, and then manually look through them and
/// pluck out the events that we care about.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create a client to use:
    let api = OnlineClient::<PolkadotConfig>::new().await?;

    // Subscribe to (in this case, finalized) blocks.
    let mut block_sub = api.blocks().subscribe_finalized().await?;

    // While this subscription is active, balance transfers are made somewhere:
    tokio::task::spawn({
        let api = api.clone();
        async move {
            let signer = PairSigner::new(AccountKeyring::Alice.pair());
            let mut transfer_amount = 1_000_000_000;

            // Make small balance transfers from Alice to Bob in a loop:
            loop {
                let transfer_tx = polkadot::tx()
                    .balances()
                    .transfer(AccountKeyring::Bob.to_account_id().into(), transfer_amount);
                api.tx()
                    .sign_and_submit_default(&transfer_tx, &signer)
                    .await
                    .unwrap();

                tokio::time::sleep(Duration::from_secs(10)).await;
                transfer_amount += 100_000_000;
            }
        }
    });

    // Get each finalized block as it arrives.
    while let Some(block) = block_sub.next().await {
        let block = block?;

        let block_hash = block.hash();

        println!(" Block {:?}", block_hash);
        // Ask for the extrinsics for this block.
        for extrinsic in block.body().await?.extrinsics() {
            println!("  Extrinsic index {:?}", extrinsic.index());

            let decoded: Result<
                polkadot::runtime_types::polkadot_runtime::RuntimeCall,
                ExtrinsicError,
            > = extrinsic.decode();
            match decoded {
                Ok(decoded) => {
                    println!("    Decoded extrinsic: {:?}", decoded);
                }
                Err(err) => {
                    println!("    Decoded extrinsic with error: {:?}", err);
                }
            }
        }

        println!("\n");
    }

    Ok(())
}