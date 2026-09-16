#[macro_export]
macro_rules! uuid_wrapper {
    ($name: ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        pub(crate) struct $name(uuid::Uuid);

        impl $name {
            pub(crate) fn as_uuid(&self) -> &uuid::Uuid{
                &self.0
            }
        }

        impl From<uuid::Uuid> for $name {
            fn from(value: uuid::Uuid) -> Self {
                Self(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<&$name> for egui::WidgetText {
            fn from(value: &$name) -> Self {
                value.to_string().into()
            }
        }

    };
}
