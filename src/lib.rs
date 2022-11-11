#![no_std]
use soroban_auth::{verify, Identifier, Signature};
use soroban_sdk::{contractimpl, symbol, Address, Bytes, Env};

pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn change_val(e: Env, key: Bytes, value: Identifier) {
        let stored_addr = e
            .data()
            .get(key.clone())
            .unwrap_or_else(|| Ok(Identifier::Contract(e.current_contract())))
            .unwrap();

        if stored_addr == Identifier::Contract(e.current_contract()) {
            e.data().set(key, value);
        } else {
            let invoker_id = match e.invoker() {
                Address::Account(id) => Identifier::Account(id),
                Address::Contract(id) => Identifier::Contract(id),
            };

            if stored_addr != invoker_id {
                panic!("you are not allowed to change this value")
            }

            e.data().set(key, value)
        }
    }

    pub fn use_sig(e: Env, sig: Signature, key: Bytes, value: Identifier) {
        let stored_addr = e
            .data()
            .get(key.clone())
            .unwrap_or_else(|| Ok(Identifier::Contract(e.current_contract())))
            .unwrap();

        if stored_addr == Identifier::Contract(e.current_contract()) {
            e.data().set(key, value);
        } else {
            if stored_addr != sig.identifier(&e) {
                panic!("you are not allowed to change this value")
            }

            verify(&e, &sig, symbol!("change"), (key.clone(), value.clone()));

            e.data().set(key, value)
        }
    }

    pub fn get(e: Env, key: Bytes) -> Identifier {
        e.data()
            .get(key)
            .unwrap_or_else(|| panic!("Key does not exist"))
            .unwrap()
    }
}

#[cfg(test)]
mod test;
