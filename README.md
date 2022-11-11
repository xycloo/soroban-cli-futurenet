# Soroban CLI basics: deployment, playing with more complex invocations, and signatures


# Introduction
If you have already tried out soroban, you are probably already familiar with Rust tests. While tests are a good way of simulating scenarios, when it comes down to actually deploying and invoking a contract you'll have to leave the comfort of Rust tests and learn how the soroban CLI works. In this simple tutorial, I'll cover some of the basics of the Soroban CLI.

# Writing a simple contract
For this article, I have created a contract that is simple but that is also a good example for explaining stuff like passing custom types to the CLI.

So what does this contract do?

### Contract workflow
The contract has two functions:

- `change_val(key: Bytes, value: Address)`, which creates or modifies a contract data entry that has `Bytes` as key, and an `Address` as value. Additionally, the value for the key can only be modified if the invoker is the address in stored as the value.
- `get(key: Bytes)`, which is a simple getter which returns the address associated with the provided `key`.

## Writing the contract
Assuming that you are already familiar with the basics of Soroban smart contracts and remembering what we said in the contract's workflow, you should be able to understand the following code:

```rust
#![no_std]
use soroban_auth::{verify, Identifier, Signature};
use soroban_sdk::{contractimpl, symbol, AccountId, Address, Bytes, Env};

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
	
    pub fn get(e: Env, key: Bytes) -> Identifier {
        e.data()
            .get(key)
            .unwrap_or_else(|| panic!("Key does not exist"))
            .unwrap()
    }
}

#[cfg(test)]
mod test;

```

Below is a successful invocation made with unit testing:

```rust
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
```

Let's say we now want to deploy and then invoke this contract on a local network or futurenet, just like in this test, how would that work?

# Futurenet/Standalone Network: Deploying and Invoking
Before deploying we need to build the WASM binary:

```bash
cargo +nightly build \
    --target wasm32-unknown-unknown \
    --release \
    -Z build-std=std,panic_abort \
    -Z build-std-features=panic_immediate_abort
```

We can now deploy the WASM binary:
```bash
> soroban deploy \
--wasm target/wasm32-unknown-unknown/release/test_soroban_cli_futurenet.wasm --secret-key DEPLOYER_SECRET --rpc-url http://INSTANCE_HOST:8000/soroban/rpc --network-passphrase 'Standalone Network ; February 2017'

output:
CONTRACT_ID
```

## Invoking change\_val

Now that we have the contract ID, we can invoke the `change_val` function:

```bash
soroban invoke \
  --id CONTRACT_ID \
  --secret-key INVOKER_SECRET \
  --rpc-url http://INSTANCE_HOST:8000/soroban/rpc \
  --network-passphrase 'Standalone Network ; February 2017' \
  --fn change_val --arg "48656c6c6f20576f726c64"\
  --arg '{"object":{"vec":[{"symbol":"Account"},{"object":{"accountId":{"publicKeyTypeEd25519":"5baa8f1a7526268d1faff4b04177800a5b323f00bc3d27fb6c33833e10d0518d"}}}]}}'

output:
  success
  null
```

The only thing that is not straightforward here is the last argument, the `Identifier`. How do we go from a stellar public key like GAVZ3QP2PV2ZXOM72C5VQYTSOO4YCQLS3VXQBERL337OUDOKUFMUFOVR, to `{"object":{"vec":[{"symbol":"Account"},{"object":{"accountId":{"publicKeyTypeEd25519":"5baa8f1a7526268d1faff4b04177800a5b323f00bc3d27fb6c33833e10d0518d"}}}]}}`?

The first thing to think of is going from GAVZ3QP2PV2ZXOM72C5VQYTSOO4YCQLS3VXQBERL337OUDOKUFMUFOVR to 5baa8f1a7526268d1faff4b04177800a5b323f00bc3d27fb6c33833e10d0518d: since stellar keys are strencoded, we need to strdecode it and then turn the obtained buffer into a hex string. 
To strdecode GAVZ3QP2PV2ZXOM72C5VQYTSOO4YCQLS3VXQBERL337OUDOKUFMUFOVR we can use the `rs-stellar-strkey` crate.

```rust
use stellar_strkey::*;

fn test_strkey_decode() {
	std::println!(
        "{:?}",
        Strkey::from_string("GAVZ3QP2PV2ZXOM72C5VQYTSOO4YCQLS3VXQBERL337OUDOKUFMUFOVR")
    );
}
```

This will return a buffer: `Ok(PublicKeyEd25519(StrkeyPublicKeyEd25519([91, 170, 143, 26, 117, 38, 38, 141, 31, 175, 244, 176, 65, 119, 128, 10, 91, 50, 63, 0, 188, 61, 39, 251, 108, 51, 131, 62, 16, 208, 81, 141])))`. 

Now we can use this simple [script](https://go.dev/play/p/z3yJPp72K8B) to turn this buffer into a hex string, which returns `Encoded Hex String:  5baa8f1a7526268d1faff4b04177800a5b323f00bc3d27fb6c33833e10d0518d`.

Now that we know what is the id, we need to construct a valid type using the CLI's JSON specification for XDR conversion.

We need to transform `Identifier::Account(account_id)` into a valid parseable JSON. By default, enums are objects that contain an array of length 1 which contains the symbol containing the enum's variant name. So if we had something like `Identifier::Account`, it would become `{"object":{"vec":[{"symbol":"Account"}]}}`. But in our case, we need to represent a tuple variant, so the array will have a length of two: first the symbol with the variant name, and then the value of the tuple. Given that an `AccountId` looks like this: `{"object":{"accountId":{"publicKeyTypeEd25519":"id"}}}`, the `Identifier::Account(AccountId)` variant will look like this:

```json
{
  "object": {
    "vec": [
      {
        "symbol": "Account"
      },
      {
        "object": {
          "accountId": {
            "publicKeyTypeEd25519": ID
          }
        }
      }
    ]
  }
}
```

So we just replace ID with the previously obtained hex value and we are golden! Also note that we pass bytes with a hex string (which is easier to share).

## Invoking get
Invoking get is very simple:

```bash
soroban invoke \                                        
  --id dbaf122037cfe68a0d47345efa8480bfbe05c0a31a48bb9f6ca63b5508a338c0 \
  --secret-key SECRET \
  --rpc-url http://INSTANCE_HOST:8000/soroban/rpc \
  --network-passphrase 'Standalone Network ; February 2017' \
  --fn get --arg "48656c6c6f20576f726c64"   

output:
  success
  ["Account","GBN2VDY2OUTCNDI7V72LAQLXQAFFWMR7AC6D2J73NQZYGPQQ2BIY2O7X"]
```

# The invoker signature
What if our contract had a function that does the same thing as `change_val` but relies on a `Signature` rather than on the invoker? That way anyone with a valid signature can invoke the function without the need of being the invoker:

```rust
#![no_std]
use soroban_auth::{verify, Identifier, Signature};
use soroban_sdk::{contractimpl, symbol, AccountId, Address, Bytes, Env};

pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
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
}

#[cfg(test)]
mod test;

```

Here what happens is that the fn takes one parameter more, which is the signature, then asserts that the identifier of the signature is in fact the identifier we want to modify, and lastly verifies that the signature is indeed correct.

A working test looks like this:
```rust
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
```

How would we invoke such a function from the CLI? The challenge is that we need to pass `Signature::Invoker`, fortunately, if you paid attention in the invoking `change_val` paragraph, you should be able to do this. In fact, since `Signature::Invoker` in a unit variant, we can simply write it like this: `{"object":{"vec":[{"symbol":"Invoker"}]}}`:

```bash
soroban invoke \
  --id 2857ffc7348724e4edc257073ac30ff75613225b67c9cfe5b949c6b8d368e8ad \
  --secret-key SECRET \
  --rpc-url http://INSTANCE_HOST:8000/soroban/rpc \
  --network-passphrase 'Standalone Network ; February 2017' \
  --fn use_sig --arg '{"object":{"vec":[{"symbol":"Invoker"}]}}' --arg "48656c6c6f20576f726c64"\
  --arg '{"object":{"vec":[{"symbol":"Account"},{"object":{"accountId":{"publicKeyTypeEd25519":"5baa8f1a7526268d1faff4b04177800a5b323f00bc3d27fb6c33833e10d0518d"}}}]}}'
```

# Conclusion

I hope this was a good way of showcasing the basics behind almost evreything CLI-related: deploying, simple invocations, custom structures and  simple authentication. If you have any questions or notice any bugs please open an issue on this repo. 
