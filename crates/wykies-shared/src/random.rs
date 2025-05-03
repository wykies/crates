use rand::distr::SampleString as _;

pub fn random_string(len: usize) -> String {
    rand::distr::Alphanumeric.sample_string(&mut rand::rng(), len)
}

pub fn random_string_def_len() -> String {
    random_string(argon2::RECOMMENDED_SALT_LEN)
}
