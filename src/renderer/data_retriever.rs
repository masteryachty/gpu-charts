use super::data_store::DataStore;
use bytemuck::cast_slice;
use js_sys::{ArrayBuffer, Uint8Array};
use reqwasm::http::Request;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wgpu::util::DeviceExt;
// use wgpu::Buffer;

// --- Structures for API Header ---

#[derive(serde::Deserialize)]
pub struct ColumnMeta {
    pub name: String,
    // pub record_size: usize,
    // pub num_records: usize,
    pub data_length: usize,
}

#[derive(serde::Deserialize)]
pub struct ApiHeader {
    pub columns: Vec<ColumnMeta>,
}

pub fn create_gpu_buffer_from_vec_f32(
    device: &wgpu::Device,
    data: &Vec<f32>,
    label: &str,
) -> wgpu::Buffer {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: cast_slice(data),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    buffer
}

// --- GPU Buffer Creation ---

pub fn create_chunked_gpu_buffer_from_arraybuffer(
    device: &wgpu::Device,
    data: &ArrayBuffer,
    label: &str,
) -> Vec<wgpu::Buffer> {
    // Convert the ArrayBuffer to a Uint8Array for byte–level access.
    let typed_array = Uint8Array::new(data);
    let total_length = typed_array.length() as usize;
    // We'll copy up to 128 MB per buffer.
    let max_chunk_size = 134217728_usize;
    let mut offset = 0_usize;
    let mut buffer_vec = Vec::new();

    while offset < total_length {
        let chunk_size = std::cmp::min(max_chunk_size, total_length - offset);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: chunk_size as u64,
            usage: wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });

        {
            // Copy the relevant slice from the JS memory into the GPU–buffer.
            let mut mapped_range = buffer.slice(0..(chunk_size as u64)).get_mapped_range_mut();
            unsafe {
                typed_array
                    .subarray(offset as u32, (offset + chunk_size) as u32)
                    .raw_copy_to_ptr(mapped_range.as_mut_ptr());
            }
        }

        buffer.unmap();
        buffer_vec.push(buffer);
        offset += chunk_size;
    }

    buffer_vec
}

// --- New API Fetching Function ---

/// Fetches the API response, then splits it into a header and binary data.
/// The response is assumed to have a header JSON (terminated by a newline, ASCII 10)
/// followed immediately by binary data.
pub async fn fetch_api_response(url: &str) -> Result<(ApiHeader, ArrayBuffer), js_sys::Error> {
    let resp = Request::get(url)
        .send()
        .await
        .map_err(|e| js_sys::Error::new(&format!("Fetch failed: {:?}", e)))?;
    let array_buffer: ArrayBuffer = JsFuture::from(resp.as_raw().array_buffer()?)
        .await
        .map(|v| v.unchecked_into::<ArrayBuffer>())
        .map_err(|e| js_sys::Error::new(&format!("ArrayBuffer conversion failed: {:?}", e)))?;

    // Create a Uint8Array view of the full ArrayBuffer.
    let uint8 = Uint8Array::new(&array_buffer);
    let mut header_end: Option<u32> = None;
    for i in 0..uint8.length() {
        if uint8.get_index(i) == 10 {
            header_end = Some(i);
            break;
        }
    }
    let header_end = header_end.ok_or(js_sys::Error::new("Header newline not found"))?;

    // Extract header bytes and convert to a UTF-8 string.
    let header_bytes = uint8.slice(0, header_end);
    let header_string = String::from_utf8(header_bytes.to_vec())
        .map_err(|e| js_sys::Error::new(&format!("UTF-8 conversion failed: {:?}", e)))?;

    // Parse the header JSON.
    let api_header: ApiHeader = serde_json::from_str(&header_string)
        .map_err(|e| js_sys::Error::new(&format!("JSON parse failed: {:?}", e)))?;

    // The binary data starts after the newline.
    let total_length = uint8.length();
    let binary_data = array_buffer.slice_with_end(header_end + 1, total_length);
    Ok((api_header, binary_data))
}

// --- (Optional) Old fetch_and_upload for compatibility ---
// This version simply fetches binary data (using the new API function)
// and creates GPU buffers from it.
// pub async fn fetch_and_upload(
//     device: &wgpu::Device,
//     url: &str,
//     label: &str,
// ) -> (ArrayBuffer, Vec<Buffer>) {
//     let (_header, binary_buffer) = fetch_api_response(url).await.unwrap();

//     let gpu_buffers = create_chunked_gpu_buffer_from_arraybuffer(device, &binary_buffer, label);
//     (binary_buffer, gpu_buffers)
// }

// --- Updated fetch_data using the New API ---

/// Fetches data from the new API endpoint and splits the returned binary data
/// into per–column buffers according to the header metadata. It then creates GPU buffers
/// for each column and adds them to the DataStore.
/// In this example we assume the two columns are "time" and "best_bid", which we map
/// to x– and y–data groups, respectively.
pub async fn fetch_data(
    device: &wgpu::Device,
    topic: &str,
    start: u32,
    end: u32,
    data_store: Rc<RefCell<DataStore>>,
) {
    // Construct the API URL.
    // For example, if topic is "BTC-USD", the URL might look like:
    //   https://localhost:8443/api/data?symbol=BTC-USD&type=MD&start=0&end=1739785500000&columns=time,best_bid
    let url = format!(
        "https://localhost:8443/api/data?symbol={}&type=MD&start={}&end={}&columns=time,best_bid",
        topic,
        start.to_string().as_str(),
        end.to_string().as_str()
    );

    let (api_header, binary_buffer) = fetch_api_response(&url).await.unwrap();

    // Split the binary data into separate ArrayBuffers for each column.
    let mut offset = 0u32;
    let mut column_buffers = std::collections::HashMap::new();
    // let uint8 = Uint8Array::new(&binary_buffer);
    // let total_length = uint8.length();

    for column in &api_header.columns {
        let data_length = column.data_length as u32;
        let start = offset;
        let end = offset + data_length;
        offset = end;

        // Slice the binary_buffer for this column.
        let col_buffer = binary_buffer.slice_with_end(start, end);
        let gpu_buffers =
            create_chunked_gpu_buffer_from_arraybuffer(device, &col_buffer, &column.name);
        column_buffers.insert(column.name.clone(), (col_buffer, gpu_buffers));
    }

    // Here we assume that the API returns columns named "time" and "best_bid".
    // Map them to x and y, respectively.
    let (x_buffer, x_gpu_buffers) = column_buffers.remove("time").unwrap();
    let (y_buffer, y_gpu_buffers) = column_buffers.remove("best_bid").unwrap();

    log::info!("xbuffer {:?}", x_buffer);

    data_store.borrow_mut().add_data_group(
        (x_buffer, x_gpu_buffers),
        (y_buffer, y_gpu_buffers),
        true,
        start,
        end,
    );
}

// --- (Optional) Old fetch_binary for backwards compatibility ---

// pub async fn fetch_binary(url: &str) -> Result<ArrayBuffer, js_sys::Error> {
//     let resp = Request::get(url)
//         .send()
//         .await
//         .map_err(|e| js_sys::Error::new(&format!("Fetch failed: {:?}", e)))?;
//     JsFuture::from(resp.as_raw().array_buffer()?)
//         .await
//         .map(|v| v.unchecked_into())
//         .map_err(|e| js_sys::Error::new(&format!("ArrayBuffer conversion failed: {:?}", e)))
// }
