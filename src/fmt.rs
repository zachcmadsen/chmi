use tracing::{Level, Subscriber};
use tracing_subscriber::{
    fmt::{self, format, FormatEvent, FormatFields},
    registry::LookupSpan,
};

pub struct Formatter(());

impl Formatter {
    pub fn new() -> Formatter {
        Formatter(())
    }
}

impl<S, N> FormatEvent<S, N> for Formatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &fmt::FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        write!(writer, "chmi: ")?;

        let level = match *event.metadata().level() {
            Level::TRACE => "trace",
            Level::DEBUG => "debug",
            Level::INFO => "info",
            Level::WARN => "warn",
            Level::ERROR => "error",
        };
        write!(writer, "{}: ", level)?;

        ctx.format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}
