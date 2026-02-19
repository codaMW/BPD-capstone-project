# BPD Capstone Project - Bitcoin Network & Chain Analytics Dashboard

## Overview

You will build a **Bitcoin statistics dashboard** backed by a **data collection + storage + API** stack. The dashboard will show:

* **On-chain history statistics** (fees, transaction counts, hashrate estimates, mining concentration)
* **Live mempool statistics**
* **Live P2P node dashboard** (peer connections, bytes sent/received, client version ratios)
* (Advanced) **Network crawler** that samples reachable Bitcoin nodes and estimates client distribution

This is a systems project: you will run a Bitcoin node, collect data, compute metrics, store it, and present it.

---

## Learning Goals

By the end, you should be able to:

* Run and query **Bitcoin Core** via RPC (and optionally ZMQ)
* Explain what mempool, blocks, fees, feerates, difficulty, and hashrate estimates mean
* Measure peer-level network behavior from a node’s perspective
* Design a small data platform: collectors → database → API → dashboard UI
* Understand measurement limits (sampling bias, reachability, spoofed user agents)

---

## What You Are Building

### Core deliverable (MVP)

A web dashboard that displays:

1. **Block History Metrics**
   * Average fee and average feerate over time (rolling windows)
   * Transaction counts per block (rolling windows)
   * Network hashrate estimate
2. **Live Mempool Metrics**
   * Total mempool size (tx count)
   * Mempool vbytes
   * Total mempool fees
   * A basic fee histogram / backlog estimate 
3. **Live P2P Metrics (from your node’s perspective)**
   * Current peer list (inbound/outbound, user agent string, protocol version)
   * Real-time bytes sent/received deltas
   * 24h peer stats: connection churn, unique peers, bandwidth totals
   * Client ratio chart based on peer `subver` strings

### Advanced deliverables (choose at least one)

* **P2P crawler** that samples reachable nodes and estimates client distribution
* **Mining concentration**: attribute blocks to pools by coinbase tags + concentration stats
* **HODLFlood**: “BTC unmoved in 10 years” via UTXO age tracking
* “BTC volume in flow” using a clear definition (raw output volume, coin-days destroyed, etc.)

---

## Non-Goals / Constraints

* You are **not** measuring the entire Bitcoin network perfectly.
* Peer “rankings” and client ratios are **only** from what you can observe:

    * Your own node’s connected peers, and/or
    * A **sample** of reachable nodes found by your crawler
* Some nodes are hidden (non-listening, Tor/I2P-only, firewalled), and user agents can be spoofed.
* Do **not** expose raw peer IP addresses in public dashboards. Aggregate or hash if needed.

---

## Architecture Requirements

Your system must have 4 layers:

1. **Bitcoin Core node**
2. **Collectors / ingestors**
3. **Storage**
4. **API + Dashboard UI**

A typical structure:

```
/collector
  chain_collector.py
  mempool_collector.py
  peer_collector.py
  crawler.py (optional)
 /api
  server.py (REST + websocket/SSE)
 /ui
  dashboard frontend
 /db
  schema.sql
 docker-compose.yml (optional)
 README.md
```

---

## Installing Bitcoin Core

* Install Bitcoin Core on your machine or a server.
* Run in **testnet** first (recommended for development), then mainnet if you have resources.

Minimum config goals:

* Enable **RPC**
* Maintain a **mempool**
* Keep enough block history to compute stats (avoid aggressive pruning until you know what you’re doing)

Example `bitcoin.conf` (you will adapt it):

```
server=1
rpcuser=YOURUSER
rpcpassword=YOURPASS
rpcport=8332

txindex=1
zmqpubrawblock=tcp://127.0.0.1:28332
zmqpubrawtx=tcp://127.0.0.1:28333
```

> Notes:
>
> * ZMQ is optional but recommended for live updates.
> * `txindex=1` is helpful for deeper analytics but increases storage needs.

## Database Considerations

You must store:

* Block-level metrics (per block and/or per day aggregates)
* Mempool snapshots (time-series)
* Peer snapshots (time-series)
* Optional crawler results (node samples, subver counts)
* Optional UTXO-age store (for HODL)

---

## Metrics Spec

### Block History

For each block height:

* `height`
* `time`
* `tx_count`
* `avg_fee` (sats)
* `avg_feerate` (sat/vB)
* `total_fees` (sats) (optional but strongly recommended)
* `difficulty` (optional)
* `hashrate_estimate` (from node RPC, or compute)

**Implementation hint:** Prefer using Bitcoin Core block stats RPC rather than decoding every tx in early milestones.

### Mempool

Snapshot every N seconds (suggest 10s):

* `timestamp`
* `mempool_tx_count`
* `mempool_vbytes`
* `mempool_total_fees`
* `min_relay_feerate` (if available)
* `feerate_histogram` (optional in MVP, required for advanced mempool)

### P2P Peers

Snapshot every 5–10 seconds:

* `timestamp`
* `peer_id` (internal)
* `inbound` boolean
* `addr` (do not display raw IP publicly)
* `subver` user agent string
* `version`
* `bytes_sent_total`, `bytes_recv_total`
* `bytes_sent_delta`, `bytes_recv_delta` (computed)
* `ping` / `minping` if available
* connection age / last send/recv times if available

### Client Ratios

At minimum, compute a chart of peers grouped by parsed `subver`, e.g.:

* Bitcoin Core
* btcd
* bcoin
* unknown/other

---

## Milestones

### Milestone 1 — MVP Dashboard

* Bitcoin Core running + RPC reachable
* Collector pulls:
    * block stats for last N blocks
    * mempool aggregate stats
    * peer list + bytes sent/recv
* API serves:
    * latest snapshot endpoints
    * historical endpoints (at least last 24h for mempool + peers)
* UI shows:
    * live tiles (mempool + peer count)
    * at least 3 charts: avg feerate, mempool vbytes, bytes in/out over time
    * peer table with subver + bandwidth deltas

**Definition of done:** dashboard updates without manual refresh and doesn’t crash after running 2+ hours.

### Milestone 2 — 24h Time-Series + Analytics

* Store snapshots and build “last 24h” views:

    * bandwidth totals
    * connection churn (connect/disconnect counts)
    * unique peers/day
    * client ratio chart over time (optional)

### Milestone 3 — Choose an Advanced Track

Pick at least one:

**Track A: Crawler**

* Sample reachable nodes using a seed list
* Handshake to collect `subver` and service bits
* Display sample size and timestamp
* Show client distribution with disclaimers

**Track B: Mining Concentration**

* Attribute blocks to pools via coinbase tags (heuristic)
* Concentration chart (top N share, HHI, unknown share)

**Track C: HODLFlood**

* Build UTXO age store
* Compute “unmoved ≥10y” total and trend

---

## Required Dashboard Pages

1. **Overview**
   * Latest block info
   * Hashrate estimate
   * Mempool size + fee state
2. **Blocks**
   * Charts: avg fee, avg feerate, tx count per block (rolling)
3. **Mempool**
   * Time-series: tx count, vbytes, total fees
   * Fee histogram / backlog estimate (if implemented)
4. **P2P Network (Local View)**
   * Peer table: inbound/outbound, subver, bytes deltas, ping
   * 24h charts: bytes in/out, peer count, churn
5. **Crawler / Network Sampling** (optional)
   * Sampled node count
   * Client distribution
   * Network type distribution (ipv4/ipv6/onion) if you implement addrv2 parsing

---

## Ethics, Safety, and Rate Limits

* Your crawler must be polite:

    * Use strict connection limits (e.g., ≤ 5 concurrent handshakes)
    * Backoff on failures
    * Respect timeouts
* Do not DDoS the network.
* Do not publish a list of raw IPs.
* Your dashboard should show **aggregated** data for public-facing pages.

---

## Suggested Implementation Approach

### Collector loop intervals

* Peers: every **5–10 seconds**
* Mempool: every **10 seconds**
* Blocks: every **1–5 minutes** (and on new block events if using ZMQ)

### Data model strategy

* Store raw snapshots in time-series tables
* Store derived aggregates (hourly/daily) in separate tables or materialized views

### Reorg handling

* Always treat the last ~6 blocks as “unstable”
* If tip changes, recompute recent window

---

## Testing Checklist

* [ ] Node RPC connectivity test script
* [ ] Collector runs for 1 hour without crashing
* [ ] Database contains expected rows after 1 hour
* [ ] API endpoints return within <500ms locally (reasonable target)
* [ ] Dashboard loads and updates live tiles
* [ ] Charts render correctly for last 24h
* [ ] Peer deltas are correct (monotonic totals → deltas computed)

---

## Deliverables

1. A Git repo with:
   * Source code (collector, API, UI)
   * Database schema/migrations
   * Setup instructions for running the node + your system
   * A short “metrics definitions” doc (what you computed and how)
2. A short demo video or live demo checklist:
   * Show mempool updates
   * Show peer table updating
   * Show block charts and hashrate estimate
3. A final report (2–4 pages):
   * Architecture diagram
   * Key metrics and what they mean
   * Limitations of your measurements
   * What you would do next

---

## Project Extensions (Optional)

* Add alerts (mempool spike, peer churn anomalies)
* Add a “What does this mean?” tooltip per metric (teaching-first UI)
* Add comparisons between:
    * fee levels vs confirmation time estimate
    * miner concentration vs time
* Add export endpoints (CSV/JSON)

---

## FAQ

**Q: Can I build this without a crawler?**
Yes. The MVP uses your node’s peer view and on-chain/mempool stats.

**Q: Why can’t we “see the whole network”?**
The Bitcoin P2P network is not fully observable. Many nodes are non-listening or behind privacy networks. Any crawler is a sample.

**Q: Do we need mainnet?**
Not for the MVP. Testnet is fine for development. Mainnet is better for “real” mempool behavior and richer history, but heavier.

---

