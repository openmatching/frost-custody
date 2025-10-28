use poem_openapi::payload::Json;
use poem_openapi::{ApiResponse, Object};

#[derive(Debug, Object)]
pub struct UnlockRequest {
    pub pin: String,
}

#[derive(Debug, Object)]
pub struct UnlockResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Object)]
pub struct LockStatusResponse {
    pub locked: bool,
    pub provider_type: String,
}

#[derive(Debug, Object)]
pub struct UnlockErrorResponse {
    pub error: String,
}
#[derive(ApiResponse)]
pub enum UnlockResult {
    #[oai(status = 200)]
    Ok(Json<UnlockResponse>),
    #[oai(status = 400)]
    BadRequest(Json<UnlockErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<UnlockErrorResponse>),
}

#[derive(ApiResponse)]
pub enum LockStatusResult {
    #[oai(status = 200)]
    Ok(Json<LockStatusResponse>),
}

// Add to UnifiedApi in dkg_api.rs:
//
// /// Unlock HSM with PIN
// #[oai(path = "/api/unlock", method = "post")]
// async fn unlock_hsm(&self, req: Json<UnlockRequest>) -> UnlockResult {
//     // Implementation in dkg_api.rs
// }
