#[test]
fn init_is_idempotent() {
    unison_log::init();
    unison_log::init();
    log::info!(target: "test::init", "smoke test info line");
}

#[test]
fn set_filter_updates_max_level() {
    unison_log::init();
    unison_log::set_filter("error");
    log::info!(target: "test::filter", "should be filtered out");
    unison_log::set_filter("debug");
    log::debug!(target: "test::filter", "should pass through");
}
