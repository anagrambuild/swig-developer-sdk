use serde_json::json;

use super::RoleAuthority;

#[test]
fn role_authority_preserves_backend_fields_and_aliases() -> Result<(), Box<dyn std::error::Error>> {
    for value in [
        json!({"authorityType": 3, "authorityValue": "arbitrary-text", "metadata": true}),
        json!({"authority_type": 3, "authority_value": "arbitrary-text"}),
    ] {
        let authority: RoleAuthority = serde_json::from_value(value)?;
        assert_eq!(authority.authority_type, 3);
        assert_eq!(authority.authority_value, "arbitrary-text");
        assert_eq!(
            serde_json::to_value(authority)?,
            json!({"authorityType": 3, "authorityValue": "arbitrary-text"})
        );
    }
    Ok(())
}

#[test]
fn role_authority_decoding_leaves_domain_validation_to_the_sdk(
) -> Result<(), Box<dyn std::error::Error>> {
    for (authority_type, authority_value) in [
        (0, String::new()),
        (9, "unknown-authority".to_owned()),
        (-1, "unknown-authority".to_owned()),
        (1, "not-base58".to_owned()),
        (3, bs58::encode([4; 33]).into_string()),
        (5, bs58::encode([5; 33]).into_string()),
        (7, bs58::encode([42; 41]).into_string()),
    ] {
        let decoded: RoleAuthority = serde_json::from_value(json!({
            "authorityType": authority_type,
            "authorityValue": authority_value,
        }))?;
        assert_eq!(decoded.authority_type, authority_type);
        assert_eq!(decoded.authority_value, authority_value);
    }
    Ok(())
}

#[test]
fn role_authority_rejects_missing_mistyped_and_conflicting_fields() {
    for value in [
        json!({}),
        json!({"authorityType": 1}),
        json!({"authorityValue": "value"}),
        json!({"authorityType": null, "authorityValue": "value"}),
        json!({"authorityType": "1", "authorityValue": "value"}),
        json!({"authorityType": 1.5, "authorityValue": "value"}),
        json!({"authorityType": 2147483648_u64, "authorityValue": "value"}),
        json!({"authorityType": 1, "authorityValue": null}),
        json!({"authorityType": 1, "authorityValue": 17}),
        json!({"authorityType": 1, "authorityValue": {}}),
        json!({"authorityType": 1, "authority_type": 1, "authorityValue": "value"}),
        json!({"authorityType": 1, "authorityValue": "value", "authority_value": "value"}),
    ] {
        assert!(serde_json::from_value::<RoleAuthority>(value).is_err());
    }
}
