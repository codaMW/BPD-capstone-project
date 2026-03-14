mod api;
mod collectors;
mod db;
mod rpc;

use tracing::info;

const RPC_URL:  &str = "http://127.0.0.1:18443";
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";
const DB_PATH:  &str = "bitcoin_dashboard.sqlite";
const ADDR:     &str = "0.0.0.0:8080";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("bitcoin_dashboard=info,warn")
        .init();

    info!("Bitcoin Dashboard starting…");

    let rpc = rpc::RpcClient::new(RPC_URL, RPC_USER, RPC_PASS);

    match rpc.blockchain_info().await {
        Ok(i) => info!("Node: {} chain, height {}", i.chain, i.blocks),
        Err(e) => { tracing::error!("Cannot reach node: {e}"); std::process::exit(1); }
    }

    let db = db::open(DB_PATH)?;

    tokio::spawn(collectors::run_chain_collector(rpc.clone(), db.clone()));
    tokio::spawn(collectors::run_mempool_collector(rpc.clone(), db.clone()));
    tokio::spawn(collectors::run_peer_collector(rpc.clone(), db.clone()));

    let app = api::router(api::AppState { db, rpc });
    info!("Dashboard → http://{ADDR}");
    let listener = tokio::net::TcpListener::bind(ADDR).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
