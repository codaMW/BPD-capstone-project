use crate::db::{self, BlockRow, Db, MempoolRow, PeerRow};
use crate::rpc::RpcClient;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

fn now_ts() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

// ── Chain collector ────────────────────────────────────────────────────────

pub async fn run_chain_collector(rpc: RpcClient, db: Db) {
    info!("Chain collector started");
    // Run immediately on start, then every 60s
    loop {
        if let Err(e) = collect_blocks(&rpc, &db).await {
            error!("Chain collector: {e:#}");
        }
        sleep(Duration::from_secs(60)).await;
    }
}

async fn collect_blocks(rpc: &RpcClient, db: &Db) -> anyhow::Result<()> {
    let tip = rpc.block_count().await?;
    let stored = db::max_block_height(db)?.unwrap_or(0);
    // Re-process last 6 for reorg safety
    let start = if stored > 6 { stored - 5 } else { 0 };

    info!("Collecting blocks {} → {}", start, tip);
    for height in start..=tip {
        match rpc.block_stats(height).await {
            Ok(bs) => {
                let pool = get_pool_name(rpc, &bs.blockhash).await;
                let row = BlockRow {
                    height: bs.height, hash: bs.blockhash,
                    time: bs.time, tx_count: bs.txs,
                    avg_fee: bs.avgfee, avg_feerate: bs.avgfeerate,
                    total_fee: bs.totalfee, min_feerate: bs.minfeerate,
                    max_feerate: bs.maxfeerate, pool_name: pool,
                };
                if let Err(e) = db::upsert_block(db, &row) {
                    warn!("store block {height}: {e}");
                }
            }
            Err(e) => warn!("getblockstats({height}): {e}"),
        }
        sleep(Duration::from_millis(30)).await;
    }
    info!("Block sync complete at height {tip}");
    Ok(())
}

/// Extract pool name from coinbase scriptSig
async fn get_pool_name(rpc: &RpcClient, hash: &str) -> Option<String> {
    let block = rpc.get_block_verbose(hash).await.ok()?;
    let cb_hex = block["tx"][0]["vin"][0]["coinbase"].as_str()?;
    let bytes = hex::decode(cb_hex).ok()?;
    let text = String::from_utf8_lossy(&bytes);

    let patterns: &[(&str, &str)] = &[
        ("AntPool", "AntPool"), ("F2Pool", "F2Pool"),
        ("ViaBTC", "ViaBTC"), ("Foundry USA", "Foundry"),
        ("SlushPool", "slush"), ("Binance Pool", "Binance"),
        ("BTC.com", "BTC.com"), ("Poolin", "Poolin"),
        ("Luxor", "Luxor"), ("MARA Pool", "MARA"),
        ("SpiderPool", "Spider"), ("BitFury", "BitFury"),
        ("Ocean", "/ocean/"), ("SBI Crypto", "SBI"),
    ];
    for (name, pat) in patterns {
        if text.contains(pat) {
            return Some(name.to_string());
        }
    }
    // Regtest / unknown
    Some("Solo/Unknown".to_string())
}

// ── Mempool collector ──────────────────────────────────────────────────────

pub async fn run_mempool_collector(rpc: RpcClient, db: Db) {
    info!("Mempool collector started");
    loop {
        if let Err(e) = collect_mempool(&rpc, &db).await {
            error!("Mempool collector: {e:#}");
        }
        sleep(Duration::from_secs(10)).await;
    }
}

async fn collect_mempool(rpc: &RpcClient, db: &Db) -> anyhow::Result<()> {
    let info = rpc.mempool_info().await?;
    db::insert_mempool(db, &MempoolRow {
        ts: now_ts(),
        tx_count: info.size,
        vbytes: info.bytes,
        total_fee: info.total_fee,
        min_fee_rate: info.mempoolminfee,
    })?;
    info!("Mempool: {} txs, {} vbytes, {:.8} BTC fees",
          info.size, info.bytes, info.total_fee);
    Ok(())
}

// ── Peer collector ─────────────────────────────────────────────────────────

pub async fn run_peer_collector(rpc: RpcClient, db: Db) {
    info!("Peer collector started");
    let mut prev: HashMap<i64, (u64, u64)> = HashMap::new();
    loop {
        if let Err(e) = collect_peers(&rpc, &db, &mut prev).await {
            error!("Peer collector: {e:#}");
        }
        sleep(Duration::from_secs(10)).await;
    }
}

async fn collect_peers(
    rpc: &RpcClient, db: &Db,
    prev: &mut HashMap<i64, (u64, u64)>,
) -> anyhow::Result<()> {
    let peers = rpc.peer_info().await?;
    let ts = now_ts();

    let rows: Vec<PeerRow> = peers.iter().map(|p| {
        let (ps, pr) = prev.get(&p.id).copied().unwrap_or((0, 0));
        let ds = if ps > 0 { Some((p.bytessent as i64 - ps as i64).max(0)) } else { None };
        let dr = if pr > 0 { Some((p.bytesrecv as i64 - pr as i64).max(0)) } else { None };
        PeerRow {
            ts, peer_id: p.id,
            addr_hash: obfuscate_addr(&p.addr),
            inbound: p.inbound, subver: p.subver.clone(),
            version: p.version,
            bytes_sent: p.bytessent, bytes_recv: p.bytesrecv,
            bytes_sent_delta: ds, bytes_recv_delta: dr,
            ping: if p.pingtime > 0.0 { Some(p.pingtime * 1000.0) } else { None },
        }
    }).collect();

    // Update prev map
    for p in &peers { prev.insert(p.id, (p.bytessent, p.bytesrecv)); }
    prev.retain(|id, _| peers.iter().any(|p| p.id == *id));

    if !rows.is_empty() {
        db::insert_peer_snapshot(db, &rows)?;
        info!("Peers: {} connected", rows.len());
    } else {
        info!("No peers connected (regtest normal)");
    }
    Ok(())
}

fn obfuscate_addr(addr: &str) -> String {
    // Strip port, mask middle octets for privacy
    let parts: Vec<&str> = addr.rsplitn(2, ':').collect();
    let port = parts.first().unwrap_or(&"0");
    let ip = parts.get(1).unwrap_or(&"?");
    if ip.contains('.') {
        let octs: Vec<&str> = ip.split('.').collect();
        let last = octs.last().copied().unwrap_or("?");
        format!("*.*.*{}:{}", last, port)
    } else {
        format!("[ipv6]:{}", port)
    }
}
