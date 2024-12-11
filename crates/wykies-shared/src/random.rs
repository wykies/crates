use rand::distributions::DistString as _;

pub fn random_string(len: usize) -> String {
    rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), len)
}

pub fn random_string_def_len() -> String {
    random_string(argon2::RECOMMENDED_SALT_LEN)
}
