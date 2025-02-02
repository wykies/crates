#[macro_export]
macro_rules! enum_repr_u8_impls {
    ($name: ident, $error_name: ident) => {
        #[derive(Debug, thiserror::Error, PartialEq, Eq)]
        pub enum $error_name {
            #[error("Invalid discriminant for {name}: {0}", name = stringify!($name))]
            InvalidDiscriminant(u8),
        }

        #[cfg(feature = "server_only")]
        impl sqlx::Encode<'_, sqlx::MySql> for $name {
            fn encode_by_ref(
                &self,
                buf: &mut <sqlx::MySql as sqlx::Database>::ArgumentBuffer<'_>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                let x = *self as u8;
                x.encode_by_ref(buf)
            }
        }

        #[cfg(feature = "server_only")]
        impl sqlx::Type<sqlx::MySql> for $name {
            fn type_info() -> <sqlx::MySql as sqlx::Database>::TypeInfo {
                u8::type_info()
            }
        }

        #[cfg(feature = "server_only")]
        impl actix_web::error::ResponseError for $error_name {
            fn status_code(&self) -> actix_web::http::StatusCode {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        }

        impl From<&$name> for egui::WidgetText {
            fn from(value: &$name) -> Self {
                value.as_ref().into()
            }
        }

        impl TryFrom<u8> for $name {
            type Error = $error_name;

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                $name::from_repr(value).ok_or($error_name::InvalidDiscriminant(value))
            }
        }
    };
}
