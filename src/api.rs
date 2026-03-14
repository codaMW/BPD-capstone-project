use crate::db::{self, Db};
use crate::rpc::RpcClient;
use axum::{extract::State, http::StatusCode, response::{Html, IntoResponse, Json}, routing::get, Router};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub rpc: RpcClient,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/api/overview", get(overview))
        .route("/api/blocks", get(blocks))
        .route("/api/mempool", get(mempool_history))
        .route("/api/mempool/latest", get(mempool_latest))
        .route("/api/peers", get(peers))
        .route("/api/peers/bandwidth", get(peer_bandwidth))
        .route("/api/peers/clients", get(client_ratios))
        .route("/api/mining/pools", get(pool_concentration))
        .with_state(state)
        .layer(CorsLayer::permissive())
}

async fn dashboard() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn overview(State(s): State<AppState>) -> impl IntoResponse {
    let chain  = s.rpc.blockchain_info().await.unwrap_or_default();
    let mining = s.rpc.mining_info().await.unwrap_or_default();
    let net    = s.rpc.network_info().await.unwrap_or_default();
    let mem    = db::latest_mempool(&s.db).ok().flatten();
    let peers  = db::latest_peers(&s.db).ok().unwrap_or_default();
    let blocks = db::get_blocks(&s.db, 20).ok().unwrap_or_default();
    let pools  = db::pool_concentration(&s.db, 10).ok().unwrap_or_default();

    Json(json!({
        "chain":          chain.chain,
        "block_height":   chain.blocks,
        "difficulty":     chain.difficulty,
        "hashrate_est":   mining.networkhashps,
        "pooled_tx":      mining.pooledtx,
        "node_version":   net.subversion,
        "connections":    net.connections,
        "connections_in": net.connections_in,
        "connections_out": net.connections_out,
        "mempool":        mem,
        "peer_count":     peers.len(),
        "latest_block":   blocks.first(),
        "recent_blocks":  blocks,
        "pool_concentration": pools.iter().map(|(n,c)| json!({"pool":n,"blocks":c})).collect::<Vec<_>>(),
    }))
}

async fn blocks(State(s): State<AppState>) -> impl IntoResponse {
    match db::get_blocks(&s.db, 150) {
        Ok(b)  => Json(json!({"blocks": b})).into_response(),
        Err(e) => err(e),
    }
}

async fn mempool_history(State(s): State<AppState>) -> impl IntoResponse {
    let since = now() - 86400;
    match db::get_mempool_history(&s.db, since) {
        Ok(d)  => Json(json!({"snapshots": d})).into_response(),
        Err(e) => err(e),
    }
}

async fn mempool_latest(State(s): State<AppState>) -> impl IntoResponse {
    match db::latest_mempool(&s.db) {
        Ok(d)  => Json(json!({"mempool": d})).into_response(),
        Err(e) => err(e),
    }
}

async fn peers(State(s): State<AppState>) -> impl IntoResponse {
    match db::latest_peers(&s.db) {
        Ok(p)  => Json(json!({"peers": p})).into_response(),
        Err(e) => err(e),
    }
}

async fn peer_bandwidth(State(s): State<AppState>) -> impl IntoResponse {
    match db::peer_bandwidth_history(&s.db, now() - 86400) {
        Ok(d) => Json(json!({"history": d.iter().map(|(ts,s,r)| json!({"ts":ts,"sent":s,"recv":r})).collect::<Vec<_>>()})).into_response(),
        Err(e) => err(e),
    }
}

async fn client_ratios(State(s): State<AppState>) -> impl IntoResponse {
    match db::client_ratios(&s.db) {
        Ok(d) => Json(json!({"clients": d.iter().map(|(v,c)| json!({"subver":v,"count":c})).collect::<Vec<_>>()})).into_response(),
        Err(e) => err(e),
    }
}

async fn pool_concentration(State(s): State<AppState>) -> impl IntoResponse {
    match db::pool_concentration(&s.db, 10) {
        Ok(d) => Json(json!({"pools": d.iter().map(|(n,c)| json!({"pool":n,"blocks":c})).collect::<Vec<_>>()})).into_response(),
        Err(e) => err(e),
    }
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

fn err(e: anyhow::Error) -> axum::response::Response {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
}
