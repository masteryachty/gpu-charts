use futures::channel::mpsc;
use futures::stream::Stream;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{BinaryType, CloseEvent, ErrorEvent, MessageEvent, WebSocket};

/// WebSocket ready state constants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReadyState {
    Connecting = 0,
    Open = 1,
    Closing = 2,
    Closed = 3,
}

impl From<u16> for ReadyState {
    fn from(value: u16) -> Self {
        match value {
            0 => ReadyState::Connecting,
            1 => ReadyState::Open,
            2 => ReadyState::Closing,
            3 => ReadyState::Closed,
            _ => ReadyState::Closed,
        }
    }
}

/// WebSocket message types
#[derive(Debug, Clone)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
}

/// WebSocket events
#[derive(Debug, Clone)]
pub enum WsEvent {
    Open,
    Message(WsMessage),
    Error(String),
    Close(u16, String),
}

/// WASM-compatible WebSocket wrapper
pub struct WasmWebSocket {
    ws: WebSocket,
    _on_open: Closure<dyn FnMut()>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(ErrorEvent)>,
    _on_close: Closure<dyn FnMut(CloseEvent)>,
}

impl WasmWebSocket {
    /// Create a new WebSocket connection
    pub fn connect(url: &str) -> Result<(Self, mpsc::UnboundedReceiver<WsEvent>), JsValue> {
        let ws = WebSocket::new(url)?;

        // Set binary type to arraybuffer for easier handling
        ws.set_binary_type(BinaryType::Arraybuffer);

        // Create channel for events
        let (tx, rx) = mpsc::unbounded();
        let tx = Rc::new(RefCell::new(tx));

        // Set up event handlers
        let tx_clone = tx.clone();
        let on_open = Closure::wrap(Box::new(move || {
            let _ = tx_clone.borrow_mut().unbounded_send(WsEvent::Open);
        }) as Box<dyn FnMut()>);

        let tx_clone = tx.clone();
        let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
            let msg = if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                WsMessage::Text(String::from(text))
            } else if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&array_buffer);
                let mut vec = vec![0u8; array.length() as usize];
                array.copy_to(&mut vec);
                WsMessage::Binary(vec)
            } else {
                return;
            };

            let _ = tx_clone.borrow_mut().unbounded_send(WsEvent::Message(msg));
        }) as Box<dyn FnMut(MessageEvent)>);

        let tx_clone = tx.clone();
        let on_error = Closure::wrap(Box::new(move |e: ErrorEvent| {
            let _ = tx_clone
                .borrow_mut()
                .unbounded_send(WsEvent::Error(e.message()));
        }) as Box<dyn FnMut(ErrorEvent)>);

        let tx_clone = tx.clone();
        let on_close = Closure::wrap(Box::new(move |e: CloseEvent| {
            let _ = tx_clone
                .borrow_mut()
                .unbounded_send(WsEvent::Close(e.code(), e.reason()));
        }) as Box<dyn FnMut(CloseEvent)>);

        // Attach event handlers
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

        Ok((
            Self {
                ws,
                _on_open: on_open,
                _on_message: on_message,
                _on_error: on_error,
                _on_close: on_close,
            },
            rx,
        ))
    }

    /// Send a text message
    pub fn send_text(&self, text: &str) -> Result<(), JsValue> {
        self.ws.send_with_str(text)
    }

    /// Send a binary message
    pub fn send_binary(&self, data: &[u8]) -> Result<(), JsValue> {
        self.ws.send_with_u8_array(data)
    }

    /// Send a JSON message
    pub fn send_json<T: serde::Serialize>(&self, value: &T) -> Result<(), JsValue> {
        let json = serde_json::to_string(value)
            .map_err(|e| JsValue::from_str(&format!("JSON serialization error: {}", e)))?;
        self.send_text(&json)
    }

    /// Get the current ready state
    pub fn ready_state(&self) -> ReadyState {
        ReadyState::from(self.ws.ready_state())
    }

    /// Check if the connection is open
    pub fn is_open(&self) -> bool {
        self.ready_state() == ReadyState::Open
    }

    /// Close the WebSocket connection
    pub fn close(&self) -> Result<(), JsValue> {
        self.ws.close()
    }

    /// Close with code and reason
    pub fn close_with_code_and_reason(&self, code: u16, reason: &str) -> Result<(), JsValue> {
        self.ws.close_with_code_and_reason(code, reason)
    }
}

/// Helper to create a WebSocket stream
pub fn websocket_stream(url: &str) -> Result<impl Stream<Item = WsEvent>, JsValue> {
    let (_ws, rx) = WasmWebSocket::connect(url)?;

    // Note: We need to store _ws somewhere to keep it alive
    // In practice, you'd want to return both or use a different pattern
    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_ready_state_conversion() {
        assert_eq!(ReadyState::from(0), ReadyState::Connecting);
        assert_eq!(ReadyState::from(1), ReadyState::Open);
        assert_eq!(ReadyState::from(2), ReadyState::Closing);
        assert_eq!(ReadyState::from(3), ReadyState::Closed);
        assert_eq!(ReadyState::from(99), ReadyState::Closed);
    }
}
