use std::collections::HashMap;

use web_sys::{window, UrlSearchParams};
pub fn get_query_params() -> HashMap<String, String> {
    // Obtain the `Location` object
    let location = window().expect("should have a Window").location();

    // Extract the full query string (including the leading '?')
    let search = location.search().expect("should get location search");

    // Initialize URLSearchParams from that query string
    let params = UrlSearchParams::new_with_str(&search).expect("failed to create UrlSearchParams");

    // Create our HashMap for storing params
    let mut map = HashMap::new();

    // Iterate over all entries in the query string
    // `entries()` returns an iterator of `[JsValue, JsValue]`
    let entries = params.entries();
    for entry in js_sys::try_iter(&entries).unwrap().unwrap() {
        let pair = entry.unwrap();
        // Each `pair` is a two-element array `[key, value]`
        let key = js_sys::Reflect::get(&pair, &0.into())
            .unwrap()
            .as_string()
            .unwrap();
        let value = js_sys::Reflect::get(&pair, &1.into())
            .unwrap()
            .as_string()
            .unwrap();

        map.insert(key, value);
    }

    map
}
