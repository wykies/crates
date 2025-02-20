use super::Username;
use rand::Rng as _;
use secrecy::{ExposeSecret, SecretString};
use std::fmt::Display;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum PasswordComplexityError {
    #[error("needs at least one uppercase character")]
    NeedsUpper,
    #[error("needs at least one lowercase character")]
    NeedsLower,
    #[error("needs at least one non-alphabetic character")]
    NeedsNonAlpha,
    #[error("your password may not contain your username")]
    HasUsername,
    #[error("consecutive duplicate characters are not allowed")]
    HasConsecutiveDuplicateChars,
    #[error("minimum length of {} not met", PasswordComplexity::MIN_LENGTH)]
    IsTooShort,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct PasswordComplexity {
    pub needs_upper: bool,
    pub needs_lower: bool,
    pub needs_non_alpha: bool,
    pub has_username: bool,
    pub has_consecutive_duplicate_chars: bool,
    pub is_too_short: bool,
}
impl PasswordComplexity {
    const MIN_LENGTH: usize = 8;
    const MAX_CONSECUTIVE_SAME_CHAR: usize = 3;

    pub fn new(username: &Username, password: &SecretString) -> Self {
        let needs_upper = password.expose_secret().chars().all(|c| !c.is_uppercase());
        let needs_lower = password.expose_secret().chars().all(|c| !c.is_lowercase());
        let needs_non_alpha = !password.expose_secret().chars().any(|c| !c.is_alphabetic());
        let has_username = {
            let lower_username = username.to_string().to_lowercase();
            password
                .expose_secret()
                .to_lowercase()
                .contains(&lower_username)
        };
        let has_consecutive_duplicate_chars = password
            .expose_secret()
            .as_bytes()
            .windows(Self::MAX_CONSECUTIVE_SAME_CHAR + 1)
            .any(|window| window.iter().all(|&c| c == window[0]));
        let is_too_short = password.expose_secret().len() < Self::MIN_LENGTH;
        Self {
            needs_upper,
            needs_lower,
            needs_non_alpha,
            has_username,
            has_consecutive_duplicate_chars,
            is_too_short,
        }
    }

    pub fn does_meet_requirements(&self) -> bool {
        let Self {
            needs_upper,
            needs_lower,
            needs_non_alpha,
            has_username,
            has_consecutive_duplicate_chars,
            is_too_short,
        } = self;
        !(*needs_upper
            || *needs_lower
            || *needs_non_alpha
            || *has_username
            || *has_consecutive_duplicate_chars
            || *is_too_short)
    }

    pub fn error_list(self) -> Vec<PasswordComplexityError> {
        let mut result = vec![];
        let Self {
            needs_upper,
            needs_lower,
            needs_non_alpha,
            has_username,
            has_consecutive_duplicate_chars: has_consecutive_duplicates,
            is_too_short,
        } = self;
        if needs_upper {
            result.push(PasswordComplexityError::NeedsUpper);
        }
        if needs_lower {
            result.push(PasswordComplexityError::NeedsLower);
        }
        if needs_non_alpha {
            result.push(PasswordComplexityError::NeedsNonAlpha);
        }
        if has_username {
            result.push(PasswordComplexityError::HasUsername);
        }
        if has_consecutive_duplicates {
            result.push(PasswordComplexityError::HasConsecutiveDuplicateChars);
        }
        if is_too_short {
            result.push(PasswordComplexityError::IsTooShort);
        }
        result
    }

    pub fn generate_random_password() -> SecretString {
        let mut rng = rand::thread_rng();
        let leading_char = rng.gen_range('a'..='z');
        let mid = Uuid::new_v4();
        let non_alpha = rng.gen_range(0..10);
        let ending_char = rng.gen_range('A'..='Z');
        let pass = format!("{leading_char}{mid}{non_alpha}{ending_char}");
        pass.into()
    }
}

impl Display for PasswordComplexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err_list = self.error_list();
        if err_list.is_empty() {
            write!(f, "All password complexity criteria met")
        } else {
            let err_list = err_list
                .into_iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            write!(f, "password complexity requirements not met: {err_list}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn random_passwords_pass() {
        let password = PasswordComplexity::generate_random_password();
        let username = Username::try_from("bob".to_string()).unwrap();
        let actual = PasswordComplexity::new(&username, &password);
        assert!(actual.does_meet_requirements());
    }

    #[rstest]
    #[case::need_upper("all_lowercase", PasswordComplexityError::NeedsUpper)]
    #[case::needs_lower("ALL_UPPERCASE", PasswordComplexityError::NeedsLower)]
    #[case::needs_non_alpha("OnlyAlpha", PasswordComplexityError::NeedsNonAlpha)]
    #[case::has_username("Includes_boB'sName", PasswordComplexityError::HasUsername)]
    #[case::has_consecutive_duplicate_chars(
        "aa7BBBBBBB",
        PasswordComplexityError::HasConsecutiveDuplicateChars
    )]
    #[case::is_too_short("jUst2sh", PasswordComplexityError::IsTooShort)]
    fn constraints_work(#[case] password: String, #[case] expected: PasswordComplexityError) {
        let username = Username::try_from("bob".to_string()).unwrap();
        let password_complexity = PasswordComplexity::new(&username, &password.into());
        let actual = password_complexity.error_list();
        assert_eq!(actual, vec![expected]);
        assert!(!password_complexity.does_meet_requirements());
    }
}
