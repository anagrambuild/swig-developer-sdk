use serde_json::json;
use solana_pubkey::Pubkey;

use crate::{types::wire::permission::PolicyPermissionWire, WalletPermission};

#[test]
fn permissions_decode_policy_formats() -> Result<(), Box<dyn std::error::Error>> {
    use WalletPermission::*;

    let mint = Pubkey::new_from_array([1; 32]);
    let destination = Pubkey::new_from_array([2; 32]);
    let program_id = Pubkey::new_from_array([3; 32]);
    let cases = [
        (All, json!({"type":"All"})),
        (
            AllButManageAuthority,
            json!({"type":"AllButManageAuthority"}),
        ),
        (ManageAuthority, json!({"type":"ManageAuthority"})),
        (
            SolLimit { amount: u64::MAX },
            json!({"type":"SolLimit","amount":u64::MAX.to_string()}),
        ),
        (
            SolRecurringLimit {
                recurring_amount: 31,
                window: 32,
            },
            json!({"type":"SolRecurringLimit","recurringAmount":"31","window":"32"}),
        ),
        (
            SolDestinationLimit {
                amount: 41,
                destination,
            },
            json!({"type":"SolDestinationLimit","amount":"41","destination":destination.to_string()}),
        ),
        (
            SolRecurringDestinationLimit {
                recurring_amount: 51,
                window: 52,
                destination,
            },
            json!({"type":"SolRecurringDestinationLimit","recurringAmount":"51","window":"52","destination":destination.to_string()}),
        ),
        (
            TokenLimit { mint, amount: 61 },
            json!({"type":"TokenLimit","mint":mint.to_string(),"amount":"61"}),
        ),
        (
            TokenRecurringLimit {
                mint,
                recurring_amount: 71,
                window: 72,
            },
            json!({"type":"TokenRecurringLimit","mint":mint.to_string(),"recurringAmount":"71","window":"72"}),
        ),
        (
            TokenDestinationLimit {
                mint,
                amount: 81,
                destination,
            },
            json!({"type":"TokenDestinationLimit","mint":mint.to_string(),"amount":"81","destination":destination.to_string()}),
        ),
        (
            TokenRecurringDestinationLimit {
                mint,
                recurring_amount: 91,
                window: 92,
                destination,
            },
            json!({"type":"TokenRecurringDestinationLimit","mint":mint.to_string(),"recurringAmount":"91","window":"92","destination":destination.to_string()}),
        ),
        (
            Program { program_id },
            json!({"type":"Program","programId":program_id.to_string()}),
        ),
        (ProgramAll, json!({"type":"ProgramAll"})),
        (ProgramCurated, json!({"type":"ProgramCurated"})),
        (
            StakeLimit { amount: 121 },
            json!({"type":"StakeLimit","amount":"121"}),
        ),
        (
            StakeRecurringLimit {
                recurring_amount: 131,
                window: 132,
            },
            json!({"type":"StakeRecurringLimit","recurringAmount":"131","window":"132"}),
        ),
        (StakeAll, json!({"type":"StakeAll"})),
        (SubAccount, json!({"type":"SubAccount"})),
    ];
    for (expected, json) in cases {
        let wire: PolicyPermissionWire = serde_json::from_value(json)?;
        assert!(WalletPermission::from(wire) == expected);
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
