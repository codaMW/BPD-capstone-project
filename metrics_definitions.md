# Metrics Definitions

## Block History Metrics

Collected via `getblockstats` RPC once per minute. The last 6 blocks are
reprocessed on each cycle for reorg safety.

| Metric | Definition | Source |
|---|---|---|
| `height` | Block height in the chain | `getblockstats.height` |
| `time` | Unix timestamp of block | `getblockstats.time` |
| `tx_count` | Number of transactions in block (including coinbase) | `getblockstats.txs` |
| `avg_fee` | Mean fee across all non-coinbase txs, in satoshis | `getblockstats.avgfee` |
| `avg_feerate` | Mean feerate (fee ÷ virtual size), in sat/vByte | `getblockstats.avgfeerate` |
| `total_fee` | Sum of all fees in the block, in satoshis | `getblockstats.totalfee` |
| `min_feerate` | Lowest feerate paid in the block | `getblockstats.minfeerate` |
| `max_feerate` | Highest feerate paid in the block | `getblockstats.maxfeerate` |
| `pool_name` | Mining pool attribution via coinbase tag scan | Decoded from `getblock` coinbase hex |

**Hashrate estimate**: Sourced from `getmininginfo.networkhashps`, which Bitcoin
Core computes as `difficulty × 2^32 ÷ 600` (target block time). Represents
a 30-block rolling window estimate in hashes per second.

## Mempool Metrics

Sampled every 10 seconds via `getmempoolinfo`.

| Metric | Definition |
|---|---|
| `tx_count` | Number of transactions currently in the mempool |
| `vbytes` | Total virtual size of all mempool transactions in vBytes |
| `total_fee` | Aggregate fees of all mempool transactions in BTC |
| `min_fee_rate` | Minimum fee rate for transaction relay (sat/vByte × 1e8 = BTC/kVB) |

**Note**: These reflect this node's local mempool. Due to propagation delays,
not all network transactions may be present. Regtest mempool is only populated
when transactions are explicitly sent.

## P2P Peer Metrics

Sampled every 10 seconds via `getpeerinfo`.

| Metric | Definition |
|---|---|
| `peer_id` | Internal Bitcoin Core peer identifier |
| `addr_hash` | Obfuscated IP (last octet retained, port shown) |
| `inbound` | True if the peer initiated the connection to us |
| `subver` | Peer's user agent string e.g. `/Satoshi:24.0.0/` |
| `version` | P2P protocol version negotiated with this peer |
| `bytes_sent` | Cumulative bytes sent to this peer since connection |
| `bytes_recv` | Cumulative bytes received from this peer since connection |
| `bytes_sent_delta` | Bytes sent since previous snapshot (10s interval) |
| `bytes_recv_delta` | Bytes received since previous snapshot (10s interval) |
| `ping` | Round-trip ping time in milliseconds |

**Delta computation**: Deltas are computed in the collector by storing the
previous snapshot's cumulative totals in a HashMap keyed by `peer_id`. If a
peer reconnects with a new ID, the delta for the first snapshot is null.

## Pool Attribution (Track B)

The coinbase transaction's `scriptSig` is decoded from hex to UTF-8 bytes.
Known pool tag patterns are scanned with substring matching:

| Pool | Tag pattern |
|---|---|
| AntPool | `AntPool` |
| F2Pool | `F2Pool` |
| ViaBTC | `ViaBTC` |
| Foundry USA | `Foundry USA` |
| Ocean | `/ocean/` |
| ... | ... |

Regtest blocks mined with `generatetoaddress` have no pool tag and are
labelled `Solo/Unknown`. This is the expected result for a private regtest node.

**Concentration metric**: Blocks are grouped by `pool_name` and counted.
The UI shows a proportional bar and percentage. This is a frequency-based
HHI proxy — the more evenly distributed the blocks, the more decentralised
the mining in the observed window.
