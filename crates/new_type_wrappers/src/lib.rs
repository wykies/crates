//! Finally extracted it into a separate crate as I keep needing it over and
//! over

use std::ops::{Div, Mul};

use rust_decimal::Decimal;

mod number;
mod string;
mod uuid;

number_wrapper!(Money);
number_wrapper!(Hours);

impl Mul<Decimal> for Money {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<Decimal> for Money {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Mul<Hours> for Money {
    type Output = Self;

    fn mul(self, rhs: Hours) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Mul<Money> for Hours {
    type Output = Money;

    fn mul(self, rhs: Money) -> Self::Output {
        Money(self.0 * rhs.0)
    }
}

impl Div<Hours> for Money {
    type Output = Self;

    fn div(self, rhs: Hours) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}
