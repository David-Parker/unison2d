//! Sink for wasm32 — routes to web_sys::console::{log,warn,error,debug}_1
//! so browser DevTools level filters work correctly.

use log::{Level, Log, Metadata, Record};
use std::sync::RwLock;
use wasm_bindgen::JsValue;

use crate::filter::Filter;

pub struct WebSink {
    pub filter: RwLock<Filter>,
}

impl Log for WebSink {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter
            .read()
            .map(|f| f.enabled(metadata.target(), metadata.level()))
            .unwrap_or(true)
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let msg = format!("[{}] {}", record.target(), record.args());
        let js = JsValue::from_str(&msg);
        match record.level() {
            Level::Error => web_sys::console::error_1(&js),
            Level::Warn => web_sys::console::warn_1(&js),
            Level::Info => web_sys::console::log_1(&js),
            Level::Debug | Level::Trace => web_sys::console::debug_1(&js),
        }
    }

    fn flush(&self) {}
}
