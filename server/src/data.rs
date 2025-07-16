// main.rs

use std::time::Instant;
use std::{collections::HashMap, convert::Infallible, fs::File, sync::Arc};

use bytes::{Buf, Bytes};
use futures::stream;
use hyper::{body::Body, header, Request, Response, StatusCode};
use memmap2::Mmap;
use serde::Serialize;
use serde_json::json;
use url::form_urlencoded;

// For mlock.
use libc::{self};

// Added for multi–day date handling.
use chrono::{TimeZone, Utc};

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
}

/// Column metadata that will be sent in the JSON header.
#[derive(Serialize)]
struct ColumnMeta {
    name: String,
    record_size: usize,
    num_records: usize,
    data_length: usize,
}

// Holds the memory–mapped column along with slice boundaries.
// struct ColumnData {
//     name: String,
//     record_size: usize,
//     num_records: usize,
//     data: Arc<Mmap>,
//     offset: usize,
//     length: usize,
// }

/// A zero–copy chunk type wrapping an Arc’d mmap slice.
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

/// This implementation is required so that our stream items (of type `DataChunk`)
/// satisfy Hyper’s requirement that they are convertible into Bytes.
impl From<DataChunk> for Bytes {
    fn from(chunk: DataChunk) -> Bytes {
        match chunk {
            DataChunk::Header(b) => b,
            DataChunk::Mmap(z) => {
                // NOTE: This conversion copies the data.
                // For a true zero–copy implementation you’d need to write a custom
                // hyper::Body type (or use a helper crate) that can stream items
                // implementing Buf without converting them to Bytes.
                Bytes::copy_from_slice(z.chunk())
            }
        }
    }
}

/// Parse query parameters from the request’s query string.
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
    Ok(QueryParams {
        symbol,
        type_,
        start,
        end,
        columns,
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

/// Given a sorted slice of u32 values (the “time” column),
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

/// Asynchronously load a file and memory–map it.
pub async fn load_mmap(path: &str) -> Result<Mmap, String> {
    let path = path.to_string();
    tokio::task::spawn_blocking(move || {
        let file = File::open(&path).map_err(|e| format!("Failed to open {path}: {e}"))?;
        // Safety: we assume the file is not modified while mapped.
        let mmap = unsafe { Mmap::map(&file).map_err(|e| format!("Failed to mmap {path}: {e}"))? };

        // Optionally lock the mmap pages in memory.
        unsafe {
            let ret = libc::mlock(mmap.as_ptr().cast::<libc::c_void>(), mmap.len());
            if ret != 0 {
                eprintln!("Warning: mlock failed for {path} (errno {ret})");
            }
        }
        Ok(mmap)
    })
    .await
    .map_err(|e| format!("Task join error: {e:?}"))?
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

    println!("Query Params: {query_params:?}");

    // Build a base path using the configured data path
    // Always use /mnt/md/data as the base path
    let data_root = "/mnt/md/data".to_string();
    let base_path = format!(
        "{}/{}/{}",
        data_root, query_params.symbol, query_params.type_
    );

    // Convert the query's start and end timestamps into UTC DateTime.
    let start_dt = Utc.timestamp_opt(i64::from(query_params.start), 0).unwrap();
    let end_dt = Utc.timestamp_opt(i64::from(query_params.end), 0).unwrap();

    println!("date: {start_dt:?}, {end_dt:?}");

    // Build a list of days (inclusive) in the query range.
    let mut days = Vec::new();
    let mut current_date = start_dt.date_naive();
    while current_date <= end_dt.date_naive() {
        days.push(current_date);
        current_date = current_date.succ_opt().unwrap();
    }
    println!("Days in range: {days:?}");

    // Prepare per–column accumulators.
    let mut col_chunks: HashMap<String, Vec<ZeroCopyChunk>> = HashMap::new();
    let mut col_total_records: HashMap<String, usize> = HashMap::new();
    let mut col_total_lengths: HashMap<String, usize> = HashMap::new();

    for col in &query_params.columns {
        col_chunks.insert(col.clone(), Vec::new());
        col_total_records.insert(col.clone(), 0);
        col_total_lengths.insert(col.clone(), 0);
    }

    // Process each day in the range.
    for day in days {
        // Format the day as "DD.MM.YY"
        let date_suffix = day.format("%d.%m.%y").to_string();
        println!("Processing day: {date_suffix}");

        // Build the time file path for this day.
        let time_path = format!("{base_path}/time.{date_suffix}.bin");
        let time_mmap = match load_mmap(&time_path).await {
            Ok(mmap) => mmap,
            Err(e) => {
                println!("Warning: Could not load time file {time_path}: {e}. Skipping day.");
                continue;
            }
        };
        let time_mmap = Arc::new(time_mmap);

        // Ensure the time file length is valid (assuming each record is 8 bytes).
        if time_mmap.len() % 4 != 0 {
            println!(
                "Invalid time file length for day {}, {}.",
                date_suffix,
                time_mmap.len()
            );
            continue;
        }
        let num_time_records = time_mmap.len() / 4;
        let time_slice: &[u32] = unsafe {
            std::slice::from_raw_parts(time_mmap.as_ptr().cast::<u32>(), num_time_records)
        };
        println!(
            "Day {}: first time = {}, last time = {}",
            date_suffix,
            time_slice.first().unwrap_or(&0),
            time_slice.last().unwrap_or(&0)
        );

        let mut is_sorted = true;

        for i in 0..time_slice.len().saturating_sub(1) {
            if time_slice[i] > time_slice[i + 1] + 60 {
                println!(
                    "Unsorted jump at index {} -> {}: {} > {}",
                    i,
                    i + 1,
                    time_slice[i],
                    time_slice[i + 1]
                );
                is_sorted = false;
            }
        }

        if is_sorted {
            println!("Day {date_suffix}: time array is sorted.");
        } else {
            println!("Day {date_suffix}: time array has unsorted jumps.");
        }

        // if (!time_slice.windows(2).all(|w| w[0] <= w[1])) {
        //     println!("{:?}", time_slice);
        // }

        // Determine if this day overlaps the query time range.
        let day_first = time_slice[0];
        let day_last = time_slice[num_time_records - 1];
        if query_params.end < day_first || query_params.start > day_last {
            println!("No overlap for day {date_suffix}.");
            continue;
        }

        // Compute effective start and end times for this day.
        let effective_start = std::cmp::max(query_params.start, day_first);
        let effective_end = std::cmp::min(query_params.end, day_last);
        let start_idx = find_start_index(time_slice, effective_start);
        let end_idx = find_end_index(time_slice, effective_end);
        if start_idx >= num_time_records || start_idx > end_idx {
            println!("Invalid time slice for day {date_suffix}.");
            continue;
        }
        let day_num_records = end_idx - start_idx + 1;
        println!(
            "Day {date_suffix}: start_idx = {start_idx}, end_idx = {end_idx}, num_records = {day_num_records}"
        );

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
            let mmap = Arc::new(mmap);
            let offset = start_idx * record_size;
            let length = day_num_records * record_size;
            println!(
                "Day {date_suffix} column '{col}': record_size = {record_size}, offset = {offset}, length = {length}"
            );
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
    println!("Header JSON: {header_str}");

    // Build the stream of DataChunks.
    let mut chunks: Vec<Result<DataChunk, std::io::Error>> = Vec::new();
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
    println!("Response prepared.");

    let duration = start_time.elapsed(); // End timing
    println!("Request handled in {duration:?}");

    Ok(response)
}
