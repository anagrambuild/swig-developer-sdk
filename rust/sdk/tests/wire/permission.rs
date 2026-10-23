use serde_json::json;
use solana_pubkey::Pubkey;

use crate::{
    types::wire::permission::{AddRoleActionWire, PolicyPermissionWire},
    WalletPermission,
};

#[test]
fn permissions_decode_policy_and_encode_add_role_formats() -> Result<(), Box<dyn std::error::Error>>
{
    use WalletPermission::*;

    let mint = Pubkey::new_from_array([1; 32]);
    let destination = Pubkey::new_from_array([2; 32]);
    let program_id = Pubkey::new_from_array([3; 32]);
    let cases = [
        (All, json!({"type":"All"}), json!({"all":{}})),
        (
            AllButManageAuthority,
            json!({"type":"AllButManageAuthority"}),
            json!({"allButManageAuthority":{}}),
        ),
        (
            ManageAuthority,
            json!({"type":"ManageAuthority"}),
            json!({"manageAuthority":{}}),
        ),
        (
            SolLimit { amount: u64::MAX },
            json!({"type":"SolLimit","amount":"18446744073709551615"}),
            json!({"solLimit":{"amount":"18446744073709551615"}}),
        ),
        (
            SolRecurringLimit {
                recurring_amount: 31,
                window: 32,
            },
            json!({"type":"SolRecurringLimit","recurringAmount":"31","window":"32"}),
            json!({"solRecurringLimit":{"recurringAmount":"31","window":"32"}}),
        ),
        (
            SolDestinationLimit {
                amount: 41,
                destination,
            },
            json!({"type":"SolDestinationLimit","amount":"41","destination":destination.to_string()}),
            json!({"solDestinationLimit":{"amount":"41","destination":destination.to_string()}}),
        ),
        (
            SolRecurringDestinationLimit {
                recurring_amount: 51,
                window: 52,
                destination,
            },
            json!({"type":"SolRecurringDestinationLimit","recurringAmount":"51","window":"52","destination":destination.to_string()}),
            json!({"solRecurringDestinationLimit":{"recurringAmount":"51","window":"52","destination":destination.to_string()}}),
        ),
        (
            TokenLimit { mint, amount: 61 },
            json!({"type":"TokenLimit","mint":mint.to_string(),"amount":"61"}),
            json!({"tokenLimit":{"mint":mint.to_string(),"amount":"61"}}),
        ),
        (
            TokenRecurringLimit {
                mint,
                recurring_amount: 71,
                window: 72,
            },
            json!({"type":"TokenRecurringLimit","mint":mint.to_string(),"recurringAmount":"71","window":"72"}),
            json!({"tokenRecurringLimit":{"mint":mint.to_string(),"recurringAmount":"71","window":"72"}}),
        ),
        (
            TokenDestinationLimit {
                mint,
                amount: 81,
                destination,
            },
            json!({"type":"TokenDestinationLimit","mint":mint.to_string(),"amount":"81","destination":destination.to_string()}),
            json!({"tokenDestinationLimit":{"mint":mint.to_string(),"amount":"81","destination":destination.to_string()}}),
        ),
        (
            TokenRecurringDestinationLimit {
                mint,
                recurring_amount: 91,
                window: 92,
                destination,
            },
            json!({"type":"TokenRecurringDestinationLimit","mint":mint.to_string(),"recurringAmount":"91","window":"92","destination":destination.to_string()}),
            json!({"tokenRecurringDestinationLimit":{"mint":mint.to_string(),"recurringAmount":"91","window":"92","destination":destination.to_string()}}),
        ),
        (
            Program { program_id },
            json!({"type":"Program","programId":program_id.to_string()}),
            json!({"program":{"programId":program_id.to_string()}}),
        ),
        (
            ProgramAll,
            json!({"type":"ProgramAll"}),
            json!({"programAll":{}}),
        ),
        (
            ProgramCurated,
            json!({"type":"ProgramCurated"}),
            json!({"programCurated":{}}),
        ),
        (
            StakeLimit { amount: 121 },
            json!({"type":"StakeLimit","amount":"121"}),
            json!({"stakeLimit":{"amount":"121"}}),
        ),
        (
            StakeRecurringLimit {
                recurring_amount: 131,
                window: 132,
            },
            json!({"type":"StakeRecurringLimit","recurringAmount":"131","window":"132"}),
            json!({"stakeRecurringLimit":{"recurringAmount":"131","window":"132"}}),
        ),
        (StakeAll, json!({"type":"StakeAll"}), json!({"stakeAll":{}})),
        (
            SubAccount,
            json!({"type":"SubAccount"}),
            json!({"subAccount":{}}),
        ),
    ];
    for (permission, policy_json, role_json) in cases {
        let policy_wire: PolicyPermissionWire = serde_json::from_value(policy_json)?;
        assert!(WalletPermission::from(policy_wire) == permission);
        assert_eq!(
            serde_json::to_value(AddRoleActionWire::from(permission))?,
            role_json
        );
    }
    Ok(())
}

#[test]
fn fieldless_policy_permissions_reject_payload_fields() {
    for kind in [
        "All",
        "AllButManageAuthority",
        "ManageAuthority",
        "ProgramAll",
        "ProgramCurated",
        "StakeAll",
        "SubAccount",
    ] {
        for payload in [
            json!({"type": kind, "amount": "1"}),
            json!({"type": kind, "destination": null}),
            json!({"type": kind, "unexpected": {}}),
        ] {
            assert!(serde_json::from_value::<PolicyPermissionWire>(payload).is_err());
        }
    }
}

#[test]
fn policy_permissions_reject_wrong_formats_missing_fields_and_invalid_native_values() {
    let mint = Pubkey::new_from_array([1; 32]).to_string();
    for value in [
        json!({"type":"SolLimit","amount":"18446744073709551616"}),
        json!({"type":"SolLimit","amount":"-1"}),
        json!({"type":"SolLimit","amount":1}),
        json!({"type":"SolLimit","amount":null}),
        json!({"type":"SolRecurringLimit","recurringAmount":"1","window":"-1"}),
        json!({"type":"TokenRecurringLimit","mint":mint,"window":"1"}),
        json!({"type":"TokenLimit","mint":"invalid","amount":"1"}),
        json!({"type":"SolDestinationLimit","destination":"invalid","amount":"1"}),
        json!({"type":"Program","programId":"invalid"}),
        json!({"type":"Program"}),
        json!({"type":"Unknown"}),
        json!({"all":{}}),
        json!({"solLimit":{"amount":"1"}}),
    ] {
        assert!(serde_json::from_value::<PolicyPermissionWire>(value).is_err());
    }
}
