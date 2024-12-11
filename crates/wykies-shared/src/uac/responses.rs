use super::UserInfo;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum LoginResponse {
    Success(UserInfo),
    SuccessForcePassChange(UserInfo),
}
