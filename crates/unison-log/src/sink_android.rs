//! Sink for Android — delegates to `android_logger` which maps
//! `log::Level` to logcat priorities.

use log::{Log, Metadata, Record};
use std::sync::RwLock;

use crate::filter::Filter;

pub struct AndroidSink {
    pub filter: RwLock<Filter>,
    inner: android_logger::AndroidLogger,
}

impl AndroidSink {
    pub fn new(filter: Filter) -> Self {
        let inner = android_logger::AndroidLogger::new(
            android_logger::Config::default().with_tag("unison2d"),
        );
        Self {
            filter: RwLock::new(filter),
            inner,
        }
    }
}

impl Log for AndroidSink {
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
        let rec = Record::builder()
            .args(format_args!("{}", msg))
            .level(record.level())
            .target("unison2d")
            .build();
        self.inner.log(&rec);
    }

    fn flush(&self) {}
}
