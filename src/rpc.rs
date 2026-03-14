use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone)]
pub struct RpcClient {
    client: Client,
    url: String,
    user: String,
    pass: String,
}

impl RpcClient {
    pub fn new(url: &str, user: &str, pass: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("HTTP client"),
            url: url.to_string(),
            user: user.to_string(),
            pass: pass.to_string(),
        }
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({"jsonrpc":"1.1","method":method,"params":params,"id":1});
        let resp = self.client
            .post(&self.url)
            .basic_auth(&self.user, Some(&self.pass))
            .json(&body)
            .send()
            .await
            .with_context(|| format!("RPC '{method}' HTTP failed"))?;
        let val: Value = resp.json().await.context("parse JSON")?;
        if let Some(e) = val.get("error") {
            if !e.is_null() {
                anyhow::bail!("RPC {method}: {e}");
            }
        }
        Ok(val["result"].clone())
    }

    pub async fn block_count(&self) -> Result<u64> {
        self.call("getblockcount", json!([])).await?
            .as_u64().context("blockcount not u64")
    }

    pub async fn block_stats(&self, height: u64) -> Result<BlockStats> {
        let v = self.call("getblockstats", json!([height])).await?;
        serde_json::from_value(v).context("BlockStats")
    }

    pub async fn mempool_info(&self) -> Result<MempoolInfo> {
        let v = self.call("getmempoolinfo", json!([])).await?;
        serde_json::from_value(v).context("MempoolInfo")
    }

    pub async fn peer_info(&self) -> Result<Vec<PeerInfo>> {
        let v = self.call("getpeerinfo", json!([])).await?;
        serde_json::from_value(v).context("PeerInfo")
    }

    pub async fn network_info(&self) -> Result<NetworkInfo> {
        let v = self.call("getnetworkinfo", json!([])).await?;
        serde_json::from_value(v).context("NetworkInfo")
    }

    pub async fn blockchain_info(&self) -> Result<BlockchainInfo> {
        let v = self.call("getblockchaininfo", json!([])).await?;
        serde_json::from_value(v).context("BlockchainInfo")
    }

    pub async fn mining_info(&self) -> Result<MiningInfo> {
        let v = self.call("getmininginfo", json!([])).await?;
        serde_json::from_value(v).context("MiningInfo")
    }

    pub async fn get_block_verbose(&self, hash: &str) -> Result<Value> {
        self.call("getblock", json!([hash, 2])).await
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct BlockStats {
    pub height: u64,
    pub time: i64,
    pub txs: u64,
    #[serde(default)] pub totalfee: i64,
    #[serde(default)] pub avgfee: i64,
    #[serde(default)] pub avgfeerate: i64,
    #[serde(default)] pub minfeerate: i64,
    #[serde(default)] pub maxfeerate: i64,
    pub blockhash: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct MempoolInfo {
    #[serde(default)] pub size: u64,
    #[serde(default)] pub bytes: u64,
    #[serde(default)] pub usage: u64,
    #[serde(default)] pub total_fee: f64,
    #[serde(default)] pub mempoolminfee: f64,
    #[serde(default)] pub minrelaytxfee: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PeerInfo {
    pub id: i64,
    #[serde(default)] pub addr: String,
    #[serde(default)] pub inbound: bool,
    #[serde(default)] pub subver: String,
    #[serde(default)] pub version: u64,
    #[serde(default)] pub bytessent: u64,
    #[serde(default)] pub bytesrecv: u64,
    #[serde(default)] pub pingtime: f64,
    #[serde(default)] pub minping: f64,
    #[serde(default)] pub conntime: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct NetworkInfo {
    #[serde(default)] pub version: u64,
    #[serde(default)] pub subversion: String,
    #[serde(default)] pub connections: u64,
    #[serde(default)] pub connections_in: u64,
    #[serde(default)] pub connections_out: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct BlockchainInfo {
    #[serde(default)] pub chain: String,
    #[serde(default)] pub blocks: u64,
    #[serde(default)] pub headers: u64,
    #[serde(default)] pub difficulty: f64,
    #[serde(default)] pub mediantime: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct MiningInfo {
    #[serde(default)] pub blocks: u64,
    #[serde(default)] pub difficulty: f64,
    #[serde(default)] pub networkhashps: f64,
    #[serde(default)] pub pooledtx: u64,
    #[serde(default)] pub chain: String,
}
