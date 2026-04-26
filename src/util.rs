use crate::PLUGIN_FOLDER;

pub(super) fn init_log() -> anyhow::Result<()> {
    let path = std::env::current_dir()?
        .join("Resources/plugins/")
        .join(PLUGIN_FOLDER)
        .join("log.txt");

    simplelog::WriteLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::Config::default(),
        std::fs::File::create(path)?,
    )
    .map_err(Into::into)
}

#[macro_export]
macro_rules! fmt_uom {
    ($value:expr, $unit:ident, $fmt:literal) => {
        format!(
            $fmt,
            $value.into_format_args($unit, DisplayStyle::Abbreviation)
        )
    };
    ($value:expr, $unit:ident) => {
        format!(
            "{}",
            $value.into_format_args($unit, DisplayStyle::Abbreviation)
        )
    };
}

pub trait DurationExt {
    fn human(self) -> humantime::FormattedDuration
    where
        Self: Sized;
}

impl DurationExt for std::time::Duration {
    fn human(self) -> humantime::FormattedDuration {
        humantime::format_duration(self)
    }
}
