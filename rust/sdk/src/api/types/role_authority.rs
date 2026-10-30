use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoleAuthority {
    #[serde(alias = "authority_type")]
    pub authority_type: i32,
    #[serde(alias = "authority_value")]
    pub authority_value: String,
}

#[cfg(test)]
#[path = "../../../tests/api/role_authority.rs"]
mod tests;
