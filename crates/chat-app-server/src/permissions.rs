use wykies_shared::uac::{default_permissions, initialize_permissions};

/// Initializes the permissions may be run more than once without issue (will only have an effect the first time)
pub fn init_permissions() {
    // Set permissions and ignore if they were already set
    let _ = initialize_permissions(default_permissions());
}
