use std::{fs, path::PathBuf, sync::Arc};

use camino::Utf8PathBuf;
use crow_driver::{Driver, DriverConfig};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

fn main() {
    let filter: EnvFilter = EnvFilter::builder()
        .with_env_var("WATT_LOG")
        .with_default_directive(LevelFilter::OFF.into())
        .from_env_lossy();

    let fmt_layer = fmt::layer()
        .with_span_events(FmtSpan::ENTER)
        .with_target(false)
        .with_level(true)
        .with_line_number(true)
        .pretty();

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    let mut driver = Driver::new(DriverConfig::new(
        Utf8PathBuf::from("C:\\Users\\vyacheslav\\crow\\test\\src"),
        Utf8PathBuf::from("C:\\Users\\vyacheslav\\crow\\test\\target"),
    ));
    driver.perform_compilation();
}
