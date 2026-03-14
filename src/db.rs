use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

pub type Db = Arc<Mutex<Connection>>;

pub fn open(path: &str) -> Result<Db> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;",
    )?;
    create_schema(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS blocks (
            height      INTEGER PRIMARY KEY,
            hash        TEXT    NOT NULL,
            time        INTEGER NOT NULL,
            tx_count    INTEGER NOT NULL DEFAULT 0,
            avg_fee     INTEGER NOT NULL DEFAULT 0,
            avg_feerate INTEGER NOT NULL DEFAULT 0,
            total_fee   INTEGER NOT NULL DEFAULT 0,
            min_feerate INTEGER NOT NULL DEFAULT 0,
            max_feerate INTEGER NOT NULL DEFAULT 0,
            pool_name   TEXT
        );
        CREATE TABLE IF NOT EXISTS mempool_snapshots (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            ts           INTEGER NOT NULL,
            tx_count     INTEGER NOT NULL DEFAULT 0,
            vbytes       INTEGER NOT NULL DEFAULT 0,
            total_fee    REAL    NOT NULL DEFAULT 0,
            min_fee_rate REAL    NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_mempool_ts ON mempool_snapshots(ts);
        CREATE TABLE IF NOT EXISTS peer_snapshots (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            ts               INTEGER NOT NULL,
            peer_id          INTEGER NOT NULL,
            addr_hash        TEXT    NOT NULL,
            inbound          INTEGER NOT NULL DEFAULT 0,
            subver           TEXT    NOT NULL DEFAULT '',
            version          INTEGER NOT NULL DEFAULT 0,
            bytes_sent       INTEGER NOT NULL DEFAULT 0,
            bytes_recv       INTEGER NOT NULL DEFAULT 0,
            bytes_sent_delta INTEGER,
            bytes_recv_delta INTEGER,
            ping             REAL
        );
        CREATE INDEX IF NOT EXISTS idx_peer_ts ON peer_snapshots(ts);
    ")?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockRow {
    pub height: u64,
    pub hash: String,
    pub time: i64,
    pub tx_count: u64,
    pub avg_fee: i64,
    pub avg_feerate: i64,
    pub total_fee: i64,
    pub min_feerate: i64,
    pub max_feerate: i64,
    pub pool_name: Option<String>,
}

pub fn upsert_block(db: &Db, b: &BlockRow) -> Result<()> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO blocks
         (height,hash,time,tx_count,avg_fee,avg_feerate,total_fee,min_feerate,max_feerate,pool_name)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![b.height as i64, b.hash, b.time, b.tx_count as i64,
                b.avg_fee, b.avg_feerate, b.total_fee,
                b.min_feerate, b.max_feerate, b.pool_name],
    )?;
    Ok(())
}

pub fn get_blocks(db: &Db, limit: usize) -> Result<Vec<BlockRow>> {
    let conn = db.lock().unwrap();
    let mut s = conn.prepare(
        "SELECT height,hash,time,tx_count,avg_fee,avg_feerate,total_fee,
                min_feerate,max_feerate,pool_name
         FROM blocks ORDER BY height DESC LIMIT ?1")?;
    let rows = s.query_map(params![limit as i64], |r| Ok(BlockRow {
        height:      r.get::<_,i64>(0)? as u64,
        hash:        r.get(1)?,
        time:        r.get(2)?,
        tx_count:    r.get::<_,i64>(3)? as u64,
        avg_fee:     r.get(4)?,
        avg_feerate: r.get(5)?,
        total_fee:   r.get(6)?,
        min_feerate: r.get(7)?,
        max_feerate: r.get(8)?,
        pool_name:   r.get(9)?,
    }))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn max_block_height(db: &Db) -> Result<Option<u64>> {
    let conn = db.lock().unwrap();
    let h: Option<i64> = conn.query_row(
        "SELECT MAX(height) FROM blocks", [], |r| r.get(0))?;
    Ok(h.map(|v| v as u64))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MempoolRow {
    pub ts: i64,
    pub tx_count: u64,
    pub vbytes: u64,
    pub total_fee: f64,
    pub min_fee_rate: f64,
}

pub fn insert_mempool(db: &Db, m: &MempoolRow) -> Result<()> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO mempool_snapshots (ts,tx_count,vbytes,total_fee,min_fee_rate)
         VALUES (?1,?2,?3,?4,?5)",
        params![m.ts, m.tx_count as i64, m.vbytes as i64, m.total_fee, m.min_fee_rate],
    )?;
    Ok(())
}

pub fn get_mempool_history(db: &Db, since_ts: i64) -> Result<Vec<MempoolRow>> {
    let conn = db.lock().unwrap();
    let mut s = conn.prepare(
        "SELECT ts,tx_count,vbytes,total_fee,min_fee_rate
         FROM mempool_snapshots WHERE ts>=?1 ORDER BY ts ASC")?;
    let rows = s.query_map(params![since_ts], |r| Ok(MempoolRow {
        ts:           r.get(0)?,
        tx_count:     r.get::<_,i64>(1)? as u64,
        vbytes:       r.get::<_,i64>(2)? as u64,
        total_fee:    r.get(3)?,
        min_fee_rate: r.get(4)?,
    }))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn latest_mempool(db: &Db) -> Result<Option<MempoolRow>> {
    let conn = db.lock().unwrap();
    match conn.query_row(
        "SELECT ts,tx_count,vbytes,total_fee,min_fee_rate
         FROM mempool_snapshots ORDER BY ts DESC LIMIT 1",
        [], |r| Ok(MempoolRow {
            ts: r.get(0)?, tx_count: r.get::<_,i64>(1)? as u64,
            vbytes: r.get::<_,i64>(2)? as u64,
            total_fee: r.get(3)?, min_fee_rate: r.get(4)?,
        }))
    {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerRow {
    pub ts: i64,
    pub peer_id: i64,
    pub addr_hash: String,
    pub inbound: bool,
    pub subver: String,
    pub version: u64,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub bytes_sent_delta: Option<i64>,
    pub bytes_recv_delta: Option<i64>,
    pub ping: Option<f64>,
}

pub fn insert_peer_snapshot(db: &Db, rows: &[PeerRow]) -> Result<()> {
    let mut conn = db.lock().unwrap();
    let tx = conn.transaction()?;
    for r in rows {
        tx.execute(
            "INSERT INTO peer_snapshots
             (ts,peer_id,addr_hash,inbound,subver,version,
              bytes_sent,bytes_recv,bytes_sent_delta,bytes_recv_delta,ping)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![r.ts, r.peer_id, r.addr_hash, r.inbound as i64,
                    r.subver, r.version as i64,
                    r.bytes_sent as i64, r.bytes_recv as i64,
                    r.bytes_sent_delta, r.bytes_recv_delta, r.ping],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn latest_peers(db: &Db) -> Result<Vec<PeerRow>> {
    let conn = db.lock().unwrap();
    let latest_ts: Option<i64> = conn.query_row(
        "SELECT MAX(ts) FROM peer_snapshots", [], |r| r.get(0))?;
    let ts = match latest_ts { Some(t) => t, None => return Ok(vec![]) };
    let mut s = conn.prepare(
        "SELECT ts,peer_id,addr_hash,inbound,subver,version,
                bytes_sent,bytes_recv,bytes_sent_delta,bytes_recv_delta,ping
         FROM peer_snapshots WHERE ts=?1")?;
    let rows = s.query_map(params![ts], |r| Ok(PeerRow {
        ts: r.get(0)?, peer_id: r.get(1)?, addr_hash: r.get(2)?,
        inbound: r.get::<_,i64>(3)? != 0, subver: r.get(4)?,
        version: r.get::<_,i64>(5)? as u64,
        bytes_sent: r.get::<_,i64>(6)? as u64,
        bytes_recv: r.get::<_,i64>(7)? as u64,
        bytes_sent_delta: r.get(8)?, bytes_recv_delta: r.get(9)?,
        ping: r.get(10)?,
    }))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn peer_bandwidth_history(db: &Db, since_ts: i64) -> Result<Vec<(i64, i64, i64)>> {
    let conn = db.lock().unwrap();
    let mut s = conn.prepare(
        "SELECT ts, SUM(bytes_sent_delta), SUM(bytes_recv_delta)
         FROM peer_snapshots
         WHERE ts>=?1 AND bytes_sent_delta IS NOT NULL
         GROUP BY ts ORDER BY ts ASC")?;
    let rows = s.query_map(params![since_ts], |r| {
        Ok((r.get::<_,i64>(0)?,
            r.get::<_,i64>(1).unwrap_or(0),
            r.get::<_,i64>(2).unwrap_or(0)))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn client_ratios(db: &Db) -> Result<Vec<(String, u64)>> {
    let conn = db.lock().unwrap();
    let latest_ts: Option<i64> = conn.query_row(
        "SELECT MAX(ts) FROM peer_snapshots", [], |r| r.get(0))?;
    let ts = match latest_ts { Some(t) => t, None => return Ok(vec![]) };
    let mut s = conn.prepare(
        "SELECT subver, COUNT(*) FROM peer_snapshots WHERE ts=?1 GROUP BY subver")?;
    let rows = s.query_map(params![ts], |r| {
        Ok((r.get::<_,String>(0)?, r.get::<_,i64>(1)? as u64))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn pool_concentration(db: &Db, limit: usize) -> Result<Vec<(String, u64)>> {
    let conn = db.lock().unwrap();
    let mut s = conn.prepare(
        "SELECT COALESCE(pool_name,'Unknown'), COUNT(*) as cnt
         FROM blocks GROUP BY pool_name ORDER BY cnt DESC LIMIT ?1")?;
    let rows = s.query_map(params![limit as i64], |r| {
        Ok((r.get::<_,String>(0)?, r.get::<_,i64>(1)? as u64))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
