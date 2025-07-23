// use std::cell::RefCell;
// use std::rc::Rc;
// use std::time::Instant;

// use arrow::array::Array;
// use arrow::array::Float32Array;
// use arrow::array::Float64Array;
// use arrow::array::Int32Array;
// use arrow::array::StringArray;
// use arrow::table::Table;
// use wasm_bindgen::prelude::*;
// use wasm_bindgen::JsCast;
// use web_sys::ErrorEvent;
// use web_sys::{window, BinaryType, MessageEvent, WebSocket};

// use super::render_engine::RenderEngine;
// use crate::renderer::data_store::DataStore;
// use arrow::ipc::reader::StreamReader;
// use std::io::Cursor;
// pub struct WebSocketConnnection {
//     pub ws: WebSocket,
// }

// impl WebSocketConnnection {
//     pub async fn new(
//         url: &str,
//         data_store: Rc<RefCell<DataStore>>,
//         render_engine: Rc<RefCell<RenderEngine>>,
//     ) -> Result<WebSocketConnnection, JsValue> {
//         let ws = WebSocket::new(url)?;
//         ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

//         let onmessage_callback =
//             Self::create_onmessage_callback(data_store.clone(), render_engine.clone(), ws.clone());
//         ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
//         onmessage_callback.forget();

//         let onerror_callback = Self::create_onerror_callback();
//         ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
//         onerror_callback.forget();

//         let onopen_callback = Self::create_onopen_callback(ws.clone());
//         ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
//         onopen_callback.forget();

//         Ok(WebSocketConnnection { ws })
//     }

//     fn create_onmessage_callback(
//         data_store: Rc<RefCell<DataStore>>,
//         render_engine: Rc<RefCell<RenderEngine>>,
//         cloned_ws: WebSocket,
//     ) -> Closure<dyn FnMut(MessageEvent)> {
//         Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
//             Self::handle_message_event(
//                 e,
//                 data_store.clone(),
//                 render_engine.clone(),
//                 cloned_ws.clone(),
//             );
//         })
//     }

//     fn handle_message_event(
//         e: MessageEvent,
//         data_store: Rc<RefCell<DataStore>>,
//         render_engine: Rc<RefCell<RenderEngine>>,
//         cloned_ws: WebSocket,
//     ) {
//         if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
//             let ar = js_sys::Uint8Array::new(&abuf);

//             // Wrap the Uint8Array in a Cursor for in-memory processing
//             let cursor = Cursor::new(ar.to_vec());

//             // Create a StreamReader from the cursor
//             let mut stream_reader = match StreamReader::try_new(cursor, None) {
//                 Ok(reader) => reader,
//                 Err(err) => {
//                     log::error!("Failed to create StreamReader: {:?}", err);
//                     return;
//                 }
//             };

//             let mut cols: Vec<Vec<u8>> = Vec::new();
//             let mut record_batches: Vec<RecordBatch> = Vec::new();
//             while let Some(batch) = stream_reader.next()? {
//                 record_batches.push(batch);
//             }

//             // Combine the RecordBatches into a single Table
//             let table = Table::try_from_iter(record_batches.into_iter()).unwrap();

//             while let Some(batch) = stream_reader.next() {
//                 let record_batch = match batch {
//                     Ok(batch) => batch,
//                     Err(err) => {
//                         log::error!("Error reading RecordBatch: {:?}", err);
//                         continue;
//                     }
//                 };

//                 if cols.is_empty() {
//                     cols.resize(record_batch.num_columns(), Vec::with_capacity(4000000));
//                 }

//                 for (column_index, column) in record_batch.columns().iter().enumerate() {
//                     match column.data_type() {
//                         arrow::datatypes::DataType::Float32 => {
//                             if let Some(float_array) =
//                                 column.as_any().downcast_ref::<Float32Array>()
//                             {
//                                 let raw_data = float_array.values();

//                                 // Ensure alignment and safety
//                                 if raw_data.len() > 0 {
//                                     let raw_ptr = raw_data.as_ptr();
//                                     if raw_ptr.is_null()
//                                         || raw_ptr.align_offset(std::mem::align_of::<f32>()) != 0
//                                     {
//                                         log::error!("Unaligned or null pointer encountered");
//                                         continue;
//                                     }

//                                     let slice = unsafe {
//                                         std::slice::from_raw_parts(
//                                             raw_ptr as *const u8,
//                                             raw_data.len() * std::mem::size_of::<f32>(),
//                                         )
//                                     };
//                                     cols[column_index].extend_from_slice(slice);
//                                 }
//                             }
//                         }
//                         _ => {
//                             log::warn!("Unsupported column type: {:?}", column.data_type());
//                         }
//                     }
//                 }
//             }

//             // Store data and trigger rendering
//             {
//                 let mut ds = data_store.borrow_mut();
//                 // ds.add_data_bulk(cols, render_engine.clone());
//                 ds.add_min_max(-0., -9000., 1699431., 8000., render_engine.clone());
//             }

//             {
//                 render_engine.borrow().render().unwrap_or_else(|err| {
//                     log::error!("Render failed: {:?}", err);
//                 });
//             }

//             cloned_ws.set_binary_type(web_sys::BinaryType::Blob);
//             if let Err(err) = cloned_ws.send_with_u8_array(&[5, 6, 7, 8]) {
//                 log::error!("Error sending message: {:?}", err);
//             }
//         } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
//             log::info!("Received Blob data");
//         } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
//             log::info!("Received Text data: {:?}", txt);
//         } else {
//             log::warn!("Unknown data type received");
//         }
//     }

//     fn create_onerror_callback() -> Closure<dyn FnMut(ErrorEvent)> {
//         Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
//             Self::handle_error_event(e);
//         })
//     }

//     fn handle_error_event(e: ErrorEvent) {
//         // Handle error event
//     }

//     fn create_onopen_callback(cloned_ws: WebSocket) -> Closure<dyn FnMut()> {
//         Closure::<dyn FnMut()>::new(move || {
//             Self::handle_open_event(cloned_ws.clone());
//         })
//     }

//     fn handle_open_event(cloned_ws: WebSocket) {
//         match cloned_ws.send_with_str("ping") {
//             Ok(_) => (),
//             Err(err) => log::info!("error sending message: {:?}", err),
//         }
//         match cloned_ws.send_with_u8_array(&[0, 1, 2, 3]) {
//             Ok(_) => (),
//             Err(err) => (),
//         }
//     }
// }
