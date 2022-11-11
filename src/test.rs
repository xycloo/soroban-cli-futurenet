use crate::{ExampleContract, ExampleContractClient};
use soroban_auth::Identifier;
use soroban_sdk::{bytes, testutils::Accounts, Env};

#[test]
fn test_change_val() {
    let e = Env::default();

    let user = e.accounts().generate();

    let contract_id = e.register_contract(None, ExampleContract);
    let client = ExampleContractClient::new(&e, &contract_id);

    client.with_source_account(&user).change_val(
        &bytes!(&e, 0x68656c6c6f),
        &soroban_auth::Identifier::Account(user.clone()),
    );

    assert_eq!(
        client.get(&bytes!(&e, 0x68656c6c6f)),
        Identifier::Account(user)
    )
}

#[test]
fn test_use_sig() {
    let e = Env::default();

    let user = e.accounts().generate();

    let contract_id = e.register_contract(None, ExampleContract);
    let client = ExampleContractClient::new(&e, &contract_id);

    client.with_source_account(&user).use_sig(
        &soroban_auth::Signature::Invoker,
        &bytes!(&e, 0x68656c6c6f),
        &Identifier::Account(user.clone()),
    );

    assert_eq!(
        client.get(&bytes!(&e, 0x68656c6c6f)),
        Identifier::Account(user)
    )
}

#[test]
#[should_panic(expected = "you are not allowed to change this value")]
fn test_invalid_invoker() {
    let e = Env::default();

    let user1 = e.accounts().generate();
    let user2 = e.accounts().generate();

    let contract_id = e.register_contract(None, ExampleContract);
    let client = ExampleContractClient::new(&e, &contract_id);

    client.with_source_account(&user1).change_val(
        &bytes!(&e, 0x68656c6c6f),
        &Identifier::Account(user1.clone()),
    );

    assert_eq!(
        client.get(&bytes!(&e, 0x68656c6c6f)),
        Identifier::Account(user1)
    );

    client.with_source_account(&user2).change_val(
        &bytes!(&e, 0x68656c6c6f),
        &Identifier::Account(user2.clone()),
    );
}
