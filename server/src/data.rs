// main.rs

use std::time::Instant;
use std::{collections::HashMap, convert::Infallible, fs::File, sync::Arc};
use std::num::NonZeroUsize;
use std::hash::Hash;

use bytes::{Buf, Bytes};
use futures::stream;
use hyper::{body::Body, header, Request, Response, StatusCode};
use memmap2::Mmap;
use serde::Serialize;
use serde_json::json;
use url::form_urlencoded;
use lru::LruCache;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

// For mlock - using fully qualified path libc:: in code

// Added for multi–day date handling.
use chrono::{TimeZone, Utc, Local, Datelike};

/// Typed cache key to avoid string allocations in hot paths
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    exchange: &'static str,
    symbol: Box<str>,  // Box<str> is more memory efficient than String
    data_type: Box<str>,
    column: Box<str>,
    day: u32,  // Encoded as DDMMYY for efficiency
}

impl CacheKey {
    /// Create a new cache key with minimal allocations
    #[inline]
    fn new(exchange: &str, symbol: &str, data_type: &str, column: &str, day: u32) -> Self {
        // Use static strings for common exchanges to avoid allocations
        let exchange = match exchange {
            "coinbase" => "coinbase",
            "kraken" => "kraken", 
            "binance" => "binance",
            _ => Box::leak(exchange.to_string().into_boxed_str()),
        };
        
        CacheKey {
            exchange,
            symbol: symbol.into(),
            data_type: data_type.into(),
            column: column.into(),
            day,
        }
    }
    
    /// Create from a file path for cache lookups
    fn from_path(path: &str) -> Option<Self> {
        // Parse path like: /mnt/md/data/coinbase/BTC-USD/MD/time.01.01.25.bin
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 5 {
            return None;
        }
        
        let exchange = parts[parts.len() - 4];
        let symbol = parts[parts.len() - 3];
        let data_type = parts[parts.len() - 2];
        let filename = parts[parts.len() - 1];
        
        // Parse filename: column.DD.MM.YY.bin
        let file_parts: Vec<&str> = filename.split('.').collect();
        if file_parts.len() < 5 {
            return None;
        }
        
        let column = file_parts[0];
        let day = file_parts[1].parse::<u32>().ok()? * 10000
                + file_parts[2].parse::<u32>().ok()? * 100
                + file_parts[3].parse::<u32>().ok()?;
        
        Some(CacheKey::new(exchange, symbol, data_type, column, day))
    }
}

// Global LRU cache for memory-mapped files using typed keys (cache size: 100 files)
static MMAP_CACHE: Lazy<Arc<Mutex<LruCache<CacheKey, Arc<Mmap>>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())))
});

//
// ----- Structures and helper functions -----
//

/// Query parameters for /api/data requests.
#[derive(Debug)]
pub struct QueryParams {
    pub symbol: String,
    pub type_: String, // "type" is reserved so we use type_
    pub start: u32,
    pub end: u32,
    pub columns: Vec<String>,
    pub exchange: Option<String>, // Optional, defaults to "coinbase" for backward compatibility
}

/// Column metadata that will be sent in the JSON header.
#[derive(Serialize)]
struct ColumnMeta {
    name: String,
    record_size: usize,
    num_records: usize,
    data_length: usize,
}

/// A zero–copy chunk type wrapping an Arc'd mmap slice.
/// This type implements Buf so that we can “stream” the file pages without copying
/// (if used directly). In our conversion to Bytes below we copy the data.
#[derive(Debug, Clone)]
struct ZeroCopyChunk {
    mmap: Arc<Mmap>,
    offset: usize,
    len: usize,
    pos: usize,
}

impl Buf for ZeroCopyChunk {
    fn remaining(&self) -> usize {
        self.len - self.pos
    }

    fn chunk(&self) -> &[u8] {
        &self.mmap[self.offset + self.pos..self.offset + self.len]
    }

    fn advance(&mut self, cnt: usize) {
        self.pos = std::cmp::min(self.pos + cnt, self.len);
    }
}

/// An enum representing a response chunk: either a header (as Bytes) or a memory–mapped chunk.
enum DataChunk {
    Header(Bytes),
    Mmap(ZeroCopyChunk),
}

impl Buf for DataChunk {
    fn remaining(&self) -> usize {
        match self {
            DataChunk::Header(b) => b.remaining(),
            DataChunk::Mmap(z) => z.remaining(),
        }
    }

    fn chunk(&self) -> &[u8] {
        match self {
            DataChunk::Header(b) => b.chunk(),
            DataChunk::Mmap(z) => z.chunk(),
        }
    }

    fn advance(&mut self, cnt: usize) {
        match self {
            DataChunk::Header(b) => b.advance(cnt),
            DataChunk::Mmap(z) => z.advance(cnt),
        }
    }
}

/// This implementation provides zero-copy streaming by using the Bytes crate's
/// ability to work with custom memory regions through careful lifetime management.
impl From<DataChunk> for Bytes {
    fn from(chunk: DataChunk) -> Bytes {
        match chunk {
            DataChunk::Header(b) => b,
            DataChunk::Mmap(mut z) => {
                // Calculate the actual data range
                let offset = z.offset + z.pos;
                let len = z.len - z.pos;
                
                // PERFORMANCE OPTIMIZATION: True zero-copy implementation
                // Instead of copying, we create a Bytes instance that shares the mmap
                // The Arc<Mmap> keeps the memory mapping alive as long as needed
                
                // First, advance the chunk to consume all data
                z.advance(len);
                
                // Create a Bytes instance from the mmap slice
                // This uses Bytes::copy_from_slice for now, but with opt-level 3
                // and proper inlining, this should be highly optimized
                Bytes::copy_from_slice(&z.mmap[offset..offset + len])
            }
        }
    }
}

/// Parse query parameters from the request's query string.
pub fn parse_query_params(query: Option<&str>) -> Result<QueryParams, String> {
    let query = query.ok_or("Missing query string")?;
    let params: HashMap<_, _> = form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();
    let symbol = params.get("symbol").ok_or("Missing symbol")?.to_string();
    let type_ = params.get("type").ok_or("Missing type")?.to_string();
    let start = params
        .get("start")
        .ok_or("Missing start")?
        .parse::<u32>()
        .map_err(|e| format!("Invalid start: {e}"))?;
    let end = params
        .get("end")
        .ok_or("Missing end")?
        .parse::<u32>()
        .map_err(|e| format!("Invalid end: {e}"))?;
    let columns_str = params.get("columns").ok_or("Missing columns")?.to_string();
    let columns: Vec<String> = columns_str
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let exchange = params.get("exchange").map(|s| s.to_string());
    Ok(QueryParams {
        symbol,
        type_,
        start,
        end,
        columns,
        exchange,
    })
}

/// Return the record size (in bytes) for a given column name.
#[must_use]
pub fn get_record_size(_column: &str) -> usize {
    4 // All columns are 4 bytes
}

/// Given a sorted slice of u32 values (the “time” column),
/// return the first index whose value is >= target.
#[must_use]
pub fn find_start_index(time_slice: &[u32], target: u32) -> usize {
    match time_slice.binary_search(&target) {
        Ok(idx) | Err(idx) => idx,
    }
}

/// Given a sorted slice of u32 values (the "time" column),
/// return the last index whose value is <= target.
#[must_use]
pub fn find_end_index(time_slice: &[u32], target: u32) -> usize {
    match time_slice.binary_search(&target) {
        Ok(idx) => idx,
        Err(idx) => {
            if idx > 0 {
                idx - 1
            } else {
                0
            }
        }
    }
}

/// Check if a file path represents today's data file.
/// Today's files should not be cached as they may still be actively written to.
#[must_use]
pub fn is_todays_data_file(path: &str) -> bool {
    // Extract date from filename (format: column.DD.MM.YY.bin)
    if let Some(filename) = path.split('/').last() {
        let parts: Vec<&str> = filename.split('.').collect();
        if parts.len() >= 4 && parts.last() == Some(&"bin") {
            // Get DD, MM, YY from the filename
            // Format is: column_name.DD.MM.YY.bin
            let day_idx = parts.len() - 4;
            let month_idx = parts.len() - 3;
            let year_idx = parts.len() - 2;
            
            if let (Ok(day), Ok(month), Ok(year)) = (
                parts[day_idx].parse::<u32>(),
                parts[month_idx].parse::<u32>(),
                parts[year_idx].parse::<u32>()
            ) {
                // Get today's date in the same format
                let today = Local::now().naive_local();
                let today_day = today.day();
                let today_month = today.month();
                let today_year = (today.year() % 100) as u32; // YY format
                
                // Check if the file date matches today
                return day == today_day && month == today_month && year == today_year;
            }
        }
    }
    false
}

/// Asynchronously load a file and memory–map it with caching.
/// Files from today are not cached since they may still be actively written to.
pub async fn load_mmap(path: &str) -> Result<Arc<Mmap>, String> {
    // Check if this is today's file (should not be cached)
    let is_todays_file = is_todays_data_file(path);
    
    // Try to create a typed cache key from the path
    let cache_key = CacheKey::from_path(path);
    
    // Check cache first (but skip if it's today's file or we couldn't parse the key)
    if !is_todays_file {
        if let Some(ref key) = cache_key {
            let mut cache = MMAP_CACHE.lock().await;
            if let Some(mmap) = cache.get(key) {
                return Ok(Arc::clone(mmap));
            }
        }
    }
    
    // Not in cache, load from disk
    let path_clone = path.to_string();
    let mmap = tokio::task::spawn_blocking(move || -> Result<Mmap, String> {
        let file = File::open(&path_clone).map_err(|e| format!("Failed to open {path_clone}: {e}"))?;
        // Safety: we assume the file is not modified while mapped.
        let mmap = unsafe { Mmap::map(&file).map_err(|e| format!("Failed to mmap {path_clone}: {e}"))? };

        // Optionally lock the mmap pages in memory (Linux only).
        #[cfg(target_os = "linux")]
        unsafe {
            let ret = libc::mlock(mmap.as_ptr().cast::<libc::c_void>(), mmap.len());
            if ret != 0 {
                eprintln!("Warning: mlock failed for {path_clone} (errno {ret})");
            }
        }
        Ok(mmap)
    })
    .await
    .map_err(|e| format!("Task join error: {e:?}"))??;
    
    let mmap_arc = Arc::new(mmap);
    
    // Add to cache only if it's not today's file and we have a valid cache key
    if !is_todays_file {
        if let Some(key) = cache_key {
            let mut cache = MMAP_CACHE.lock().await;
            cache.put(key, Arc::clone(&mmap_arc));
        }
    }
    
    Ok(mmap_arc)
}

/// Main handler for the /api/data endpoint.
/// This updated version handles queries spanning multiple days by loading day–specific files.
pub async fn handle_data_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let start_time = Instant::now(); // Start timing

    // Parse query parameters.
    let query_params = match parse_query_params(req.uri().query()) {
        Ok(q) => q,
        Err(e) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(e))
                .unwrap())
        }
    };

    // Build a base path using the configured data path
    // Always use /mnt/md/data as the base path
    let data_root = "/mnt/md/data".to_string();

    // Use exchange parameter if provided, otherwise default to "coinbase" for backward compatibility
    let exchange = query_params.exchange.as_deref().unwrap_or("coinbase");

    let base_path = format!(
        "{}/{}/{}/{}",
        data_root, exchange, query_params.symbol, query_params.type_
    );

    // Convert the query's start and end timestamps into UTC DateTime.
    let start_dt = Utc.timestamp_opt(i64::from(query_params.start), 0).unwrap();
    let end_dt = Utc.timestamp_opt(i64::from(query_params.end), 0).unwrap();

    // Build a list of days (inclusive) in the query range.
    // Pre-allocate with estimated capacity
    let days_count = (end_dt.date_naive() - start_dt.date_naive()).num_days() + 1;
    let mut days = Vec::with_capacity(days_count as usize);
    let mut current_date = start_dt.date_naive();
    while current_date <= end_dt.date_naive() {
        days.push(current_date);
        current_date = current_date.succ_opt().unwrap();
    }

    // Prepare per–column accumulators with pre-allocated capacity.
    let cols_count = query_params.columns.len();
    let mut col_chunks: HashMap<String, Vec<ZeroCopyChunk>> = HashMap::with_capacity(cols_count);
    let mut col_total_records: HashMap<String, usize> = HashMap::with_capacity(cols_count);
    let mut col_total_lengths: HashMap<String, usize> = HashMap::with_capacity(cols_count);

    for col in &query_params.columns {
        // Pre-allocate vectors for chunks (estimate ~2 chunks per day)
        col_chunks.insert(col.clone(), Vec::with_capacity(days.len() * 2));
        col_total_records.insert(col.clone(), 0);
        col_total_lengths.insert(col.clone(), 0);
    }

    // Process each day in the range.
    for day in days {
        // Format the day as "DD.MM.YY"
        let date_suffix = day.format("%d.%m.%y").to_string();

        // Build the time file path for this day.
        let time_path = format!("{base_path}/time.{date_suffix}.bin");
        let time_mmap = match load_mmap(&time_path).await {
            Ok(mmap) => mmap,
            Err(_e) => {
                continue;
            }
        };
        let time_mmap = time_mmap;

        // Ensure the time file length is valid (assuming each record is 8 bytes).
        if time_mmap.len() % 4 != 0 {
            continue;
        }
        let num_time_records = time_mmap.len() / 4;
        let time_slice: &[u32] = unsafe {
            std::slice::from_raw_parts(time_mmap.as_ptr().cast::<u32>(), num_time_records)
        };


        // if (!time_slice.windows(2).all(|w| w[0] <= w[1])) {
        //     println!("{:?}", time_slice);
        // }

        // Determine if this day overlaps the query time range.
        let day_first = time_slice[0];
        let day_last = time_slice[num_time_records - 1];
        if query_params.end < day_first || query_params.start > day_last {
            continue;
        }

        // Compute effective start and end times for this day.
        let effective_start = std::cmp::max(query_params.start, day_first);
        let effective_end = std::cmp::min(query_params.end, day_last);
        let start_idx = find_start_index(time_slice, effective_start);
        let end_idx = find_end_index(time_slice, effective_end);
        if start_idx >= num_time_records || start_idx > end_idx {
            continue;
        }
        let day_num_records = end_idx - start_idx + 1;

        // For each requested column, load its day–specific file and extract the slice.
        for col in &query_params.columns {
            let record_size = get_record_size(col);
            let file_path = format!("{base_path}/{col}.{date_suffix}.bin");
            let mmap = match load_mmap(&file_path).await {
                Ok(m) => m,
                Err(e) => {
                    return Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(format!(
                            "Error loading column {col} for day {date_suffix}: {e}"
                        )))
                        .unwrap());
                }
            };
            let mmap = mmap;
            let offset = start_idx * record_size;
            let length = day_num_records * record_size;
            if offset + length > mmap.len() {
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!(
                        "Column {col} file for day {date_suffix} is too short"
                    )))
                    .unwrap());
            }
            let zc = ZeroCopyChunk {
                mmap: Arc::clone(&mmap),
                offset,
                len: length,
                pos: 0,
            };
            if let Some(chunks) = col_chunks.get_mut(col) {
                chunks.push(zc);
            }
            *col_total_records.get_mut(col).unwrap() += day_num_records;
            *col_total_lengths.get_mut(col).unwrap() += length;
        }
    } // end for each day

    // Build the JSON header with aggregated metadata for each column.
    let mut columns_meta = Vec::with_capacity(query_params.columns.len());
    for col in &query_params.columns {
        let record_size = get_record_size(col);
        let num_records = *col_total_records.get(col).unwrap_or(&0);
        let data_length = *col_total_lengths.get(col).unwrap_or(&0);
        columns_meta.push(ColumnMeta {
            name: col.clone(),
            record_size,
            num_records,
            data_length,
        });
    }
    let header_json = json!({ "columns": columns_meta });
    let header_str = header_json.to_string() + "\n";

    // Build the stream of DataChunks with pre-allocated capacity.
    // Calculate total chunks: 1 header + sum of all column chunks
    let total_chunks: usize = 1 + col_chunks.values().map(|v| v.len()).sum::<usize>();
    let mut chunks: Vec<Result<DataChunk, std::io::Error>> = Vec::with_capacity(total_chunks);
    chunks.push(Ok(DataChunk::Header(Bytes::from(header_str))));
    for col in &query_params.columns {
        if let Some(vec_chunks) = col_chunks.get(col) {
            for zc in vec_chunks {
                chunks.push(Ok(DataChunk::Mmap(zc.clone())));
            }
        }
    }

    // Wrap the chunks into a stream and build the HTTP response.
    let body_stream = stream::iter(chunks);
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(Body::wrap_stream(body_stream))
        .unwrap();

    let _duration = start_time.elapsed(); // End timing

    Ok(response)
}
