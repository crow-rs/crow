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
    let _ = miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .unicode(false)
                .rgb_colors(miette::RgbColors::Preferred)
                .show_related_errors_as_nested()
                .context_lines(3)
                .build(),
        )
    }));

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

    // let mut driver = Driver::new(DriverConfig::new(
    //     Utf8PathBuf::from("/home/vyacheslav/crow/test/src"),
    //     Utf8PathBuf::from("/home/vyacheslav/crow/test/target"),
    // ));

    
    let driver = Driver::new(DriverConfig::new(
        Utf8PathBuf::from("/home/f0rits/Documents/crow/test/src"),
        Utf8PathBuf::from("/home/f0rits/Documents/crow/test/target"),
    ));
    

    driver.compile();
}
