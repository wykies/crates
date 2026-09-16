#[macro_export]
macro_rules! number_wrapper {
    ($name: ident) => {
        #[derive(
            Debug,
            Default,
            serde::Serialize,
            serde::Deserialize,
            Clone,
            Copy,
            PartialEq,
            PartialOrd,
            Ord,
            Eq,
        )]
        pub struct $name(rust_decimal::Decimal);

        impl $name {
            pub const ZERO: Self = Self::new(rust_decimal::dec!(0));
            pub const THRESHOLD_DECIMAL_PLACES: u32 = 5;

            pub const fn new(value: rust_decimal::Decimal) -> Self {
                Self(value)
            }

            pub fn abs(&self) -> Self {
                Self(self.0.abs())
            }

            #[must_use = "method returns a new value and does not mutate the original value"]
            #[inline]

            pub fn round(self) -> Self {
                Self(self.0.round())
            }

            pub fn is_zero(&self) -> bool {
                self.0.is_zero()
            }

            pub fn as_f64(self) -> f64 {
                self.0.as_f64()
            }

            /// Returns an instance of self rounded to 2 decimal places
            #[must_use = "method returns a new value and does not mutate the original value"]
            #[inline]
            pub fn to_2_dp(self) -> Self {
                Self(self.0.round_dp(2))
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:.2}", self.to_2_dp().0)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.to_string()
            }
        }

        impl From<rust_decimal::Decimal> for $name {
            fn from(value: rust_decimal::Decimal) -> Self {
                Self(value.round_dp(Self::THRESHOLD_DECIMAL_PLACES))
            }
        }

        impl std::ops::Add for $name {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                Self(self.0 + rhs.0)
            }
        }

        impl std::ops::Sub for $name {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self::Output {
                Self(self.0 - rhs.0)
            }
        }

        impl std::ops::Mul for $name {
            type Output = Self;

            fn mul(self, rhs: Self) -> Self::Output {
                Self(self.0 * rhs.0)
            }
        }

        impl std::ops::Div for $name {
            type Output = Self;

            fn div(self, rhs: Self) -> Self::Output {
                Self(self.0 / rhs.0)
            }
        }

        impl std::ops::AddAssign<Self> for $name {
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }

        impl std::ops::SubAssign<Self> for $name {
            fn sub_assign(&mut self, rhs: Self) {
                self.0 -= rhs.0;
            }
        }

        impl std::ops::MulAssign<Self> for $name {
            fn mul_assign(&mut self, rhs: Self) {
                self.0 *= rhs.0;
            }
        }

        impl std::ops::DivAssign<Self> for $name {
            fn div_assign(&mut self, rhs: Self) {
                self.0 /= rhs.0;
            }
        }

        impl std::iter::Sum for $name {
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.fold(Self::default(), std::ops::Add::add)
            }
        }

        impl From<$name> for egui::RichText {
            fn from(value: $name) -> Self {
                value.to_string().into()
            }
        }

        impl From<&$name> for egui::WidgetText {
            fn from(value: &$name) -> Self {
                value.to_string().into()
            }
        }

        impl From<$name> for egui::WidgetText {
            fn from(value: $name) -> Self {
                value.to_string().into()
            }
        }
    };
}
