use unison_log::filter::Filter;
use unison_log::Level;

#[test]
fn parses_bare_level() {
    let f = Filter::parse("info");
    assert!(f.enabled("anything", Level::Info));
    assert!(f.enabled("anything", Level::Warn));
    assert!(f.enabled("anything", Level::Error));
    assert!(!f.enabled("anything", Level::Debug));
    assert!(!f.enabled("anything", Level::Trace));
}

#[test]
fn parses_off() {
    let f = Filter::parse("off");
    assert!(!f.enabled("anything", Level::Error));
    assert!(!f.enabled("anything", Level::Info));
}

#[test]
fn per_target_override_tightens_default() {
    let f = Filter::parse("info,unison::audio=warn");
    assert!(!f.enabled("unison::audio", Level::Info));
    assert!(f.enabled("unison::audio", Level::Warn));
    assert!(f.enabled("unison::audio", Level::Error));
    // Prefix match → child also capped at warn.
    assert!(!f.enabled("unison::audio::mixer", Level::Info));
    assert!(f.enabled("unison::audio::mixer", Level::Warn));
    // Other targets still at default.
    assert!(f.enabled("game", Level::Info));
    assert!(!f.enabled("game", Level::Debug));
}

#[test]
fn per_target_override_loosens_default() {
    let f = Filter::parse("warn,game=debug");
    assert!(f.enabled("game", Level::Debug));
    assert!(f.enabled("game", Level::Info));
    assert!(!f.enabled("other", Level::Info));
    assert!(f.enabled("other", Level::Warn));
}

#[test]
fn longest_prefix_wins() {
    let f = Filter::parse("info,unison=warn,unison::audio=debug");
    assert!(f.enabled("unison::audio::mixer", Level::Debug));
    assert!(!f.enabled("unison::physics", Level::Info));
    assert!(f.enabled("unison::physics", Level::Warn));
    assert!(f.enabled("game", Level::Info));
}

#[test]
fn malformed_does_not_panic() {
    let _ = Filter::parse("");
    let _ = Filter::parse("nonsense");
    let _ = Filter::parse("info,=warn");
    let _ = Filter::parse("info,foo=notalevel");
    let _ = Filter::parse("foo=bar=baz");
}

#[test]
fn whitespace_is_tolerated() {
    let f = Filter::parse(" info , unison::audio = warn ");
    assert!(!f.enabled("unison::audio", Level::Info));
    assert!(f.enabled("unison::audio", Level::Warn));
    assert!(f.enabled("other", Level::Info));
}

#[test]
fn case_insensitive_levels() {
    let f = Filter::parse("INFO,game=Debug");
    assert!(f.enabled("game", Level::Debug));
    assert!(f.enabled("other", Level::Info));
}

#[test]
fn unknown_level_token_falls_back_to_info() {
    let f = Filter::parse("bogus");
    assert!(f.enabled("anything", Level::Info));
    assert!(!f.enabled("anything", Level::Debug));
}

#[test]
fn default_on_empty_is_info() {
    let f = Filter::parse("");
    assert!(f.enabled("anything", Level::Info));
    assert!(!f.enabled("anything", Level::Debug));
}
