#[macro_export]
macro_rules! string_wrapper {
    ($name: ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        pub(crate) struct $name(String);

        impl TryFrom<String> for $name {
            type Error = anyhow::Error;

            fn try_from(value: String) -> anyhow::Result<Self> {
                if value.is_empty() {
                    anyhow::bail!("empty string cannot be a {}", stringify!($name));
                }
                Ok(Self(value))
            }
        }

        impl TryFrom<&str> for $name {
            type Error = anyhow::Error;

            fn try_from(value: &str) -> anyhow::Result<Self> {
                value.to_string().try_into()
            }
        }

        impl TryFrom<std::borrow::Cow<'_, str>> for $name {
            type Error = anyhow::Error;

            fn try_from(value: std::borrow::Cow<'_, str>) -> anyhow::Result<Self> {
                value.to_string().try_into()
            }
        }


        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl From<&$name> for String {
            fn from(value: &$name) -> Self {
                value.0.clone()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<&$name> for egui::WidgetText {
            fn from(value: &$name) -> Self {
                value.0.as_str().into()
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0[..]
            }
        }
    };
}
