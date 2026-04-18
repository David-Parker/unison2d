//! Sink for iOS (Xcode terminal) and native desktop — writes to stderr.

use log::{Log, Metadata, Record};
use std::sync::RwLock;

use crate::filter::Filter;

pub struct StderrSink {
    pub filter: RwLock<Filter>,
}

impl Log for StderrSink {
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
        eprintln!(
            "[{} {}] {}",
            record.level(),
            record.target(),
            record.args()
        );
    }

    fn flush(&self) {}
}
