//! Finally extracted it into a separate crate as I keep needing it over and
//! over

use std::ops::{Div, Mul};

use rust_decimal::Decimal;

mod number;
mod string;
mod string_id;
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

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used)]
    use super::*;
    use rstest::rstest;
    use rust_decimal::dec;

    #[rstest]
    #[case::zero(Money::ZERO, 0)]
    #[case::one(Money::from(dec!(1.00)), 100)]
    #[case::neg_one(Money::from(dec!(-1)), -100)]
    #[case::two_dec_places(Money::from(dec!(1.23)), 123)]
    #[case::neg_two_dec_places(Money::from(dec!(-1.23)), -123)]
    #[case::three_dec_places_round_down1(Money::from(dec!(1.231)), 123)]
    #[case::three_dec_places_round_down2(Money::from(dec!(1.234)), 123)]
    #[case::three_dec_places_round_up1(Money::from(dec!(1.235)), 124)]
    #[case::three_dec_places_round_up2(Money::from(dec!(1.239)), 124)]
    #[case::neg_three_dec_places_round_down1(Money::from(dec!(-1.231)), -123)]
    #[case::neg_three_dec_places_round_down2(Money::from(dec!(-1.234)), -123)]
    #[case::neg_three_dec_places_round_up1(Money::from(dec!(-1.235)), -124)]
    #[case::neg_three_dec_places_round_up2(Money::from(dec!(-1.239)), -124)]
    fn verify_cents_conversion(#[case] input: Money, #[case] expected: i64) {
        let actual = input.to_cents().unwrap();
        assert_eq!(actual, expected);
    }
}
