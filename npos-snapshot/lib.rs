use subxt::{client::OnlineClient, lightclient::LightClient, SubstrateConfig};

pub enum Chain {
    Secure(String),
    Insecure(String),
    Light(String),
}

pub const POLKADOT_SPEC: &str = include_str!("polkadot.json");
pub const KUSAMA_SPEC: &str = include_str!("kusama.json");
pub const WESTEND_SPEC: &str = include_str!("westend.json");

impl Chain {
    pub fn parse(chain: &str) -> Self {
        match chain {
            x if x == "polkadot" => Chain::Light(POLKADOT_SPEC.to_string()),
            x if x == "kusama" => Chain::Light(KUSAMA_SPEC.to_string()),
            x if x == "westend" => Chain::Light(WESTEND_SPEC.to_string()),
            x if x == "vara" => Chain::Secure("wss://archive-rpc.vara.network".to_string()),
            x if x == "joystream" => Chain::Secure("wss://rpc.joystream.org".to_string()),
            x if x == "enjin" => Chain::Secure("wss://rpc.relay.blockchain.enjin.io".to_string()),
            x if x == "canary" => Chain::Secure("wss://rpc.relay.canary.enjin.io".to_string()),
            x if x == "paseo" => Chain::Secure("wss://rpc.ibp.network/paseo".to_string()),

            url if url.starts_with("wss://") || url.starts_with("https://") => {
                Chain::Secure(url.to_string())
            }

            url if url.starts_with("ws://") || url.starts_with("http://") => {
                Chain::Insecure(url.to_string())
            }

            json => {
                let chain_spec = std::fs::read_to_string(json).unwrap();
                Chain::Light(chain_spec)
            }
        }
    }
    pub async fn api(&self) -> Result<OnlineClient<SubstrateConfig>, Box<dyn std::error::Error>> {
        let api = match self {
            Chain::Secure(url) => OnlineClient::<SubstrateConfig>::from_url(url).await?,
            Chain::Insecure(url) => OnlineClient::<SubstrateConfig>::from_insecure_url(url).await?,
            Chain::Light(spec) => {
                let (_lightclient, rpc) = LightClient::relay_chain(spec.to_string())?;
                OnlineClient::<SubstrateConfig>::from_rpc_client(rpc).await?
            }
        };
        Ok(api)
    }
}

pub async fn snapshot<T: subxt::Config>(
    api: &OnlineClient<T>,
    key: &str,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut key = key.to_lowercase();
    if key.starts_with("0x") {
        key = key[2..].to_string();
    }
    let raw_key: Vec<u8> = hex::decode(&key)?;
    let storage = api.storage().at_latest().await?.fetch_raw(raw_key).await?;
    if let Some(value) = storage {
        let v = hex::encode(&value);
        std::fs::write(&output, &v)?;
        eprintln!("{} bytes written to {}", v.len(), output);
        Ok(())
    } else {
        Err(format!("No storage value found for key 0x{}", key).into())
    }
}
