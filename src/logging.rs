pub extern crate fern;

#[allow(dead_code)]
pub fn default_formatter(out: fern::FormatCallback, message: &std::fmt::Arguments, record: &log::Record) {
    out.finish(format_args!(
        "{time}[{level}][{target}] {message}",
        time = chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
        level = record.level(),
        target = record.target(),
        message = message,
    ))
}

#[allow(dead_code)]
pub fn color_formatter(out: fern::FormatCallback, message: &std::fmt::Arguments, record: &log::Record) {
    out.finish(format_args!(
        "{color}{message}{color_reset}",
        message = message,
        color = format_args!("\x1B[{color_number}m", color_number = fern::colors::ColoredLevelConfig::new().get_color(&record.level()).to_fg_str()),
        color_reset = "\x1B[0m",
    ))
}