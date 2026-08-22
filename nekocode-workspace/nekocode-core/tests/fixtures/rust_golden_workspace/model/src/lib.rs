//! A valid package that exercises trait, impl, macro, cfg, and feature edges.

pub trait Summary {
    fn summary(&self) -> String;
}

pub struct Widget {
    name: &'static str,
}

impl Widget {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl Summary for Widget {
    fn summary(&self) -> String {
        format!("widget:{}", self.name)
    }
}

#[macro_export]
macro_rules! describe {
    ($value:expr) => {{
        use $crate::Summary as _;
        $value.summary()
    }};
}

#[cfg(feature = "feature_probe")]
pub const FEATURE_STATE: &str = "enabled";

#[cfg(not(feature = "feature_probe"))]
pub const FEATURE_STATE: &str = "disabled";
