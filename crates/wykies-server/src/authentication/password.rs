#[cfg(feature = "mysql")]
use crate::db_utils::db_int_to_bool;
use crate::db_utils::validate_one_row_affected;
use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use secrecy::{ExposeSecret, SecretString};
use tracing::debug;
use wykies_shared::db_types::DbPool;
use wykies_shared::{
    id::DbId,
    telemetry::spawn_blocking_with_tracing,
    uac::{AuthError, LoginResponse, Permissions, UserInfo, Username},
};

use super::LoginAttemptLimit;

pub struct Credentials {
    pub username: String,
    pub password: SecretString,
}

#[derive(Debug)]
pub struct DbUser {
    pub username: String,
    pub password_hash: SecretString,
    pub force_pass_change: bool,
    pub display_name: String,
    pub permissions: Permissions,
    pub enabled: bool,
    pub locked_out: bool,
    pub failed_attempts: i8,
}

impl DbUser {
    fn is_default(&self) -> bool {
        self.username.is_empty()
    }
}

impl Default for DbUser {
    fn default() -> Self {
        Self {
            username: Default::default(),
            password_hash: SecretString::from(
                "$argon2id$v=19$m=15000,t=2,p=1$\
            gZiV/M1gPc22ElAH/Jh1Hw$\
            CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno",
            ),
            force_pass_change: Default::default(),
            display_name: Default::default(),
            permissions: Default::default(),
            enabled: true, // Needs to be enabled to prevent non-existent users showing as disabled
            locked_out: Default::default(),
            failed_attempts: Default::default(),
        }
    }
}

#[tracing::instrument(skip(pool))]
async fn get_user_from_db(username: &str, pool: &DbPool) -> Result<Option<DbUser>, anyhow::Error> {
    #[cfg(feature = "mysql")]
    let query = sqlx::query!(
        "SELECT UserName, password_hash, ForcePassChange, DisplayName, Enabled, LockedOut, FailedAttempts, Permissions
        FROM user
        LEFT JOIN roles ON user.AssignedRole = roles.RoleID
        WHERE UserName = ?
        ",
        username,
    );

    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let query = sqlx::query!(
        "SELECT user_name, password_hash, force_pass_change, display_name, is_enabled, locked_out, failed_attempts, permissions
        FROM users
        LEFT JOIN roles ON users.assigned_role = roles.role_id
        WHERE user_name = $1;",
        username,
    );
    let row = query
        .fetch_optional(pool)
        .await
        .context("Failed to performed a query to retrieve stored credentials.")?;
    let Some(row) = row else {
        debug!("User not found: {username}");
        return Ok(None);
    };

    #[cfg(feature = "mysql")]
    return Ok(Some(DbUser {
        username: row.UserName,
        password_hash: SecretString::from(row.password_hash),
        force_pass_change: db_int_to_bool(row.ForcePassChange),
        display_name: row.DisplayName,
        permissions: row.Permissions.unwrap_or_default().try_into()?,
        enabled: db_int_to_bool(row.Enabled),
        locked_out: db_int_to_bool(row.LockedOut),
        failed_attempts: row.FailedAttempts,
    }));

    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    Ok(Some(DbUser {
        username: row.user_name,
        password_hash: SecretString::from(row.password_hash),
        force_pass_change: row.force_pass_change,
        display_name: row.display_name,
        permissions: row.permissions.try_into()?,
        enabled: row.is_enabled,
        locked_out: row.locked_out,
        failed_attempts: row.failed_attempts.try_into()?,
    }))
}

#[derive(Debug)]
pub struct AuthUserInfo {
    pub username: String,
    pub force_pass_change: bool,
    pub display_name: String,
    pub permissions: Permissions,
}

impl AuthUserInfo {
    pub fn into_login_response(self, branch_id: DbId) -> anyhow::Result<LoginResponse> {
        let username = self.username.try_into().context("username invalid")?;
        let display_name = self
            .display_name
            .try_into()
            .context("display name invalid")?;
        let user_info = UserInfo {
            username,
            display_name,
            branch_id,
            permissions: self.permissions,
        };

        if self.force_pass_change {
            Ok(LoginResponse::SuccessForcePassChange(user_info))
        } else {
            Ok(LoginResponse::Success(user_info))
        }
    }
}

/// Uses a default (empty DbUser) to make it harder to do timing attacks to find
/// usernames
#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &DbPool,
    login_attempt_limit: &LoginAttemptLimit,
) -> Result<AuthUserInfo, AuthError> {
    let mut db_user = DbUser::default();

    if let Some(x) = get_user_from_db(&credentials.username, pool).await? {
        db_user = x;
    }

    let expected_password_hash = db_user.password_hash.clone();

    let password_check_status = spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")?;

    // For timing attack reasons this must happen after checking the password
    // because for the default user it would just always return disabled instead of
    // wrong username/password
    if !db_user.enabled {
        // User is disabled
        return Err(AuthError::NotEnabled);
    }

    if db_user.locked_out {
        // user is locked out already
        return Err(AuthError::LockedOut);
    }

    // Error out if password was wrong after checking for lockout
    match password_check_status {
        Ok(()) => {
            // Password validation passed
            // Reset login attempts if applicable
            if db_user.failed_attempts > 0 {
                reset_failed_login_attempts(&db_user.username, pool).await?;
            }
        }
        Err(e) => {
            if !db_user.is_default() {
                // Only increment if not default because
                // we also get here if there was an invalid username
                // in which case the db_user will still be empty
                increment_locked_out_count(&db_user.username, pool, login_attempt_limit).await?;
            }
            return Err(e); // Return that password failed
        }
    };

    let DbUser {
        username,
        force_pass_change,
        display_name,
        permissions,
        ..
    } = db_user;

    Ok(AuthUserInfo {
        username,
        force_pass_change,
        display_name,
        permissions,
    })
}

#[tracing::instrument(skip(pool))]
async fn reset_failed_login_attempts(username: &str, pool: &DbPool) -> anyhow::Result<()> {
    #[cfg(feature = "mysql")]
    let query = sqlx::query!(
        "UPDATE `user` SET `FailedAttempts`=0 WHERE `UserName`=?;",
        username,
    );
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let query = sqlx::query!(
        "UPDATE users SET failed_attempts=0 WHERE user_name=$1;",
        username,
    );

    let sql_result = query
        .execute(pool)
        .await
        .context("failed to reset `failed attempts`")?;
    validate_one_row_affected(&sql_result).context("failed to to reset `failed attempts`")
}

#[tracing::instrument(skip(pool))]
async fn set_locked_out_in_db(username: &str, pool: &DbPool, value: bool) -> anyhow::Result<()> {
    #[cfg(feature = "mysql")]
    let query = {
        // TODO 5: Do we need the manual conversion to numbers here?
        let value = if value { 1 } else { 0 };
        sqlx::query!(
            "UPDATE `user` SET `LockedOut` = ? WHERE `user`.`UserName` = ?;",
            value,
            username,
        )
    };
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let query = sqlx::query!(
        "UPDATE users SET locked_out = $1 WHERE users.user_name = $2;",
        value,
        username,
    );
    let sql_result = query
        .execute(pool)
        .await
        .context("failed to set user to disabled")?;
    validate_one_row_affected(&sql_result).context("failed to set user to disabled")
}

#[tracing::instrument(skip(pool))]
async fn increment_locked_out_count(
    username: &str,
    pool: &DbPool,
    login_attempt_limit: &LoginAttemptLimit,
) -> Result<(), AuthError> {
    // Increment current value in DB
    #[cfg(feature = "mysql")]
    let query = sqlx::query!(
        "UPDATE `user` SET `FailedAttempts`=`FailedAttempts`+1 WHERE `UserName`=?; ",
        username,
    );
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let query = sqlx::query!(
        "UPDATE users SET failed_attempts=failed_attempts+1 WHERE user_name=$1;",
        username,
    );
    let sql_result = query
        .execute(pool)
        .await
        .context("failed to increment `failed attempts`")?;
    validate_one_row_affected(&sql_result).context("failed to to increment `failed attempts`")?;

    // Get new current value
    #[cfg(feature = "mysql")]
    let current_failed_attempts = sqlx::query!(
        "SELECT `FailedAttempts` FROM `user` WHERE `UserName`=?; ",
        username
    )
    .fetch_one(pool)
    .await
    .context("failed to get `failed attempts` count")?
    .FailedAttempts;
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let current_failed_attempts: i8 = sqlx::query!(
        "SELECT failed_attempts FROM users WHERE user_name=$1;",
        username
    )
    .fetch_one(pool)
    .await
    .context("failed to get `failed attempts` count")?
    .failed_attempts
    .try_into()
    .context("failed_attempts from db is out of range")?;

    // Check if flag needs to be toggled
    if current_failed_attempts >= login_attempt_limit.as_i8() {
        set_locked_out_in_db(username, pool, true).await?;
        return Err(AuthError::LockedOut);
    } else {
        Ok(())
    }
}

#[tracing::instrument(skip(expected_password_hash, password_candidate))]
fn verify_password_hash(
    expected_password_hash: SecretString,
    password_candidate: SecretString,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;

    if Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .is_ok()
    {
        Ok(())
    } else {
        // Invalid Password
        Err(AuthError::InvalidUserOrPassword)
    }
}

#[tracing::instrument(skip(password, pool))]
pub async fn change_password(
    username: &Username,
    password: SecretString,
    should_force_pass_change: bool,
    pool: &DbPool,
) -> anyhow::Result<()> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("failed to hash password")?;
    #[cfg(feature = "mysql")]
    let query = sqlx::query!(
        "UPDATE `user` SET 
        `password_hash` = ?, 
        `ForcePassChange` = ?,
        `PassChangeDate` = CURRENT_DATE()
        WHERE `user`.`UserName` = ?;",
        password_hash.expose_secret(),
        should_force_pass_change,
        username,
    );
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    // TODO 5: Check why encode trait impl doesn't make converting not necessary
    let query = sqlx::query!(
        "
        UPDATE users SET 
        password_hash = $1, 
        force_pass_change = $2,
        pass_change_date = CURRENT_DATE
        WHERE users.user_name = $3; 
        ",
        password_hash.expose_secret(),
        should_force_pass_change,
        username.as_ref(),
    );
    let sql_result = query
        .execute(pool)
        .await
        .context("failed to change user's password in the database.")?;
    validate_one_row_affected(&sql_result)?;

    Ok(())
}

fn compute_password_hash(password: SecretString) -> Result<SecretString, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = argon2_settings()
        .hash_password(password.expose_secret().as_bytes(), &salt)?
        .to_string();
    Ok(SecretString::from(password_hash))
}

pub fn argon2_settings() -> Argon2<'static> {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).expect("invalid parameters"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assumptions_on_default_db_user() {
        let db_user = DbUser::default();
        assert_eq!(db_user.username, "", "default username must be empty to make it easy to identify if it is still the default value");
        assert_ne!(
            db_user.password_hash.expose_secret(),
            "",
            "this should always be a valid value and empty is not a valid value"
        );
        db_user.is_default();
        assert!(
            db_user.is_default(),
            "no changes have been made this should still register as a default user"
        );
    }
}
