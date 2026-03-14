# Bitcoin Network & Chain Analytics Dashboard

A production-grade Bitcoin statistics dashboard built in **Rust**, backed by a
live Bitcoin Core node running in regtest mode.

## Architecture
```
Bitcoin Core (regtest, Docker)
        │  JSON-RPC
        ▼
  Rust Collectors (tokio async tasks)
  ├── chain_collector   — getblockstats every 60s + reorg-safe reprocessing
  ├── mempool_collector — getmempoolinfo every 10s
  └── peer_collector    — getpeerinfo every 10s, delta bandwidth computed
        │
        ▼
  SQLite (WAL mode, 4 tables)
  blocks · mempool_snapshots · peer_snapshots · (pool attribution inline)
        │
        ▼
  Axum HTTP server (REST JSON API + static file serving)
        │
        ▼
  Single-page Dashboard (HTML + Chart.js, SSE-style auto-refresh)
```

## Stack

| Layer | Technology |
|---|---|
| Node | Bitcoin Core 24.0.1 (Docker, regtest) |
| Collectors | Rust + tokio async tasks |
| Storage | SQLite via rusqlite (bundled) |
| API | Axum 0.7 (REST JSON) |
| UI | Vanilla HTML + Chart.js 4 |

## Setup

### 1. Start Bitcoin Core
```bash
docker start s2-week-1-interacting-with-a-bitcoin-node-codamw-bitcoin-1
```

Node config: `rpcbind=0.0.0.0:18443`, `rpcauth=alice:...`, `txindex=1`, regtest.

### 2. Build and run
```bash
cargo build --release
./target/release/bitcoin-dashboard
```

Dashboard available at **http://localhost:8080**

### 3. Generate data (regtest)
```bash
# Mine blocks
ADDR=$(docker exec <container> bitcoin-cli -rpcport=18443 -rpcuser=alice -rpcpassword=password getnewaddress)
docker exec <container> bitcoin-cli -rpcport=18443 -rpcuser=alice -rpcpassword=password generatetoaddress 150 "$ADDR"

# Create mempool activity
for i in $(seq 1 20); do
  docker exec <container> bitcoin-cli -rpcport=18443 -rpcuser=alice -rpcpassword=password \
    sendtoaddress "$ADDR" 0.0001
done
```

## Dashboard Pages

| Page | Content |
|---|---|
| Overview | Live tiles: height, difficulty, hashrate, mempool, peers; recent blocks table; pool concentration chart |
| Blocks | Avg feerate chart, tx count chart, full block history table |
| Mempool | Tx count / vbytes / fees time-series (24h) |
| P2P Network | Peer table (IPs obfuscated), bandwidth chart, client distribution |

## API Endpoints

| Endpoint | Description |
|---|---|
| `GET /api/overview` | Aggregated live stats |
| `GET /api/blocks` | Last 150 blocks with all metrics |
| `GET /api/mempool` | 24h mempool snapshots |
| `GET /api/mempool/latest` | Most recent mempool snapshot |
| `GET /api/peers` | Latest peer snapshot |
| `GET /api/peers/bandwidth` | 24h bandwidth deltas |
| `GET /api/peers/clients` | Peer client distribution |
| `GET /api/mining/pools` | Pool concentration from coinbase tags |

## Advanced Track Implemented

**Track B  Mining Concentration**: Coinbase scriptSig is decoded from hex and
scanned for known pool tag patterns (AntPool, F2Pool, ViaBTC, Foundry, etc.).
Attribution is stored per block and surfaced in the Overview page as a
colour-coded bar chart with percentage breakdown.

## Testing
```bash
# RPC connectivity
curl -s --user alice:password \
  --data-binary '{"jsonrpc":"1.1","method":"getblockchaininfo","params":[]}' \
  -H 'content-type: text/plain;' http://127.0.0.1:18443/

# API health
curl http://localhost:8080/api/overview | jq .block_height
curl http://localhost:8080/api/blocks | jq '.blocks | length'
curl http://localhost:8080/api/mempool/latest | jq .
```

## Limitations

- **Regtest network**: No real peers (peer stats show 0 connections, which is
  correct for an isolated regtest node). On testnet/mainnet this populates.
- **Peer sampling**: Only this node's connected peers are visible (~8–125 on
  mainnet). Hidden nodes (Tor/I2P, firewalled) are not observable.
- **User agent spoofing**: `subver` strings in peer info can be forged.
- **Genesis block**: `getblockstats` cannot compute fee data for block 0
  (no UTXO set undo data). This is a known Bitcoin Core limitation.
- **Pool attribution**: Based on coinbase tag heuristics only  pools can omit
  tags or use non-standard ones. Unknown blocks are labelled "Solo/Unknown".
- **Hashrate estimate**: Derived from `getmininginfo` which uses a 30-block
  rolling window. Not suitable for short-window regtest chains.
