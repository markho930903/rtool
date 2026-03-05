macro_rules! define_feature_keys {
    ($( $variant:ident => $key:literal ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum FeatureKey {
            $($variant),+
        }

        pub const FEATURE_KEYS: [&str; [$(stringify!($variant)),+].len()] = [$($key),+];

        impl FeatureKey {
            pub fn parse(input: &str) -> Option<Self> {
                let normalized = input.trim();
                $(
                    if normalized.eq_ignore_ascii_case($key) {
                        return Some(Self::$variant);
                    }
                )+
                None
            }

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $key),+
                }
            }
        }
    };
}

define_feature_keys!(
    AppManager => "app_manager",
    Clipboard => "clipboard",
    Launcher => "launcher",
    Locale => "locale",
    Logging => "logging",
    Screenshot => "screenshot",
    Settings => "settings",
);

#[cfg(test)]
mod tests {
    use super::{FEATURE_KEYS, FeatureKey};

    #[test]
    fn parse_should_accept_all_registered_keys() {
        for key in FEATURE_KEYS {
            assert_eq!(FeatureKey::parse(key).map(FeatureKey::as_str), Some(key));
        }
    }

    #[test]
    fn parse_should_reject_unknown_feature() {
        assert_eq!(FeatureKey::parse("user_settings"), None);
        assert_eq!(FeatureKey::parse(""), None);
    }
}
