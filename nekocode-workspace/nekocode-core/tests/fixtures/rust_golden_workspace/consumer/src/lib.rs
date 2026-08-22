use golden_model::{describe, Widget, FEATURE_STATE};

pub fn rendered_widget() -> String {
    describe!(Widget::new("nekocode"))
}

pub fn feature_state() -> &'static str {
    FEATURE_STATE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_trait_impl_macro_and_feature() {
        assert_eq!(rendered_widget(), "widget:nekocode");
        assert_eq!(feature_state(), "enabled");
    }
}
