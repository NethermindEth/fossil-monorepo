use core::ops::AddAssign;
use openzeppelin_upgrades::interface::{IUpgradeableDispatcher, IUpgradeableDispatcherTrait};
use sha2_input::{ISha2InputDispatcher, ISha2InputDispatcherTrait};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, declare, get_class_hash, map_entry_address,
    start_cheat_caller_address, start_mock_call, store,
};
use starknet::ContractAddress;
use starknet::class_hash::class_hash_const;

fn deploy_contract_sha2_input(
    owner: starknet::ContractAddress, fossil_store: starknet::ContractAddress,
) -> ContractAddress {
    let contract = declare("Sha2Input").unwrap().contract_class();
    let (contract_address, _) = contract
        .deploy(@array![owner.into(), fossil_store.into()])
        .unwrap();
    contract_address
}

fn fossil_store() -> starknet::ContractAddress {
    'FOSSIL_STORE_ADDRESS'.try_into().unwrap()
}

fn owner() -> starknet::ContractAddress {
    'OWNER_ADDRESS'.try_into().unwrap()
}

#[test]
fn test_hash_avg_fees_and_store() {
    let start_timestamp = 1714636800_u64;

    start_mock_call(
        fossil_store(), selector!("get_avg_fee"), 565966358523639806057303238395575262125865566208,
    );

    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };

    dispatcher.hash_avg_fees_and_store(start_timestamp);
    let hash_res = dispatcher.get_hash_stored_avg_fees(start_timestamp);

    assert(
        hash_res == [
            0x71316d72, 0x99f3b0a0, 0xf67978f3, 0x2f96f5de, 0x0a358b38, 0x65b4a286, 0x568c4b8a,
            0x833531ef,
        ],
        'invalid hash result',
    );
}

#[test]
#[should_panic(expected: 'Avg fees is 0')]
fn test_hash_avg_fees_and_store_fail_with_avg_fees_is_0() {
    let start_timestamp = 1714636800_u64;

    start_mock_call(fossil_store(), selector!("get_avg_fee"), 0);

    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };
    dispatcher.hash_avg_fees_and_store(start_timestamp);
}

#[test]
fn test_hash_batched_avg_fees() {
    let start_timestamp = 1714636800_u64;

    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };

    let num_in_a_batch = 32_u64; // 5760/180
    for i in 0..num_in_a_batch {
        let timestamp = start_timestamp + (i * 3600_u64 * 180_u64);

        store(
            contract_address,
            map_entry_address(
                selector!("hash_stored_avg_fees"), // storage variable name
                array![timestamp.into()].span() // map key
            ),
            array![
                1899064690, 2582884512, 4135155955, 798422494, 171281208, 1706336902, 1452034954,
                2201301487,
            ]
                .span(),
        );
    }

    dispatcher.hash_batched_avg_fees(start_timestamp);
    let hash_res = dispatcher.get_hash_stored_batched_avg_fees(start_timestamp);

    assert(
        hash_res == [
            0xa375b797, 0x375dca1e, 0x2eb59bf5, 0xe5743db6, 0x343351e6, 0xa16ea306, 0xb40c72b8,
            0xa68e3133,
        ],
        'invalid hash result',
    );
}

#[test]
#[should_panic(expected: 'Hash is empty')]
fn test_hash_batched_avg_fees_fail_with_underlying_hashes_are_not_calculated() {
    let start_timestamp = 1714636800_u64;

    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };

    dispatcher.hash_batched_avg_fees(start_timestamp);
}

#[test]
fn test_should_be_able_to_set_fossil_store_by_owner() {
    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };

    // address before
    let address_before = dispatcher.get_fossil_store();
    assert(address_before == fossil_store(), 'invalid fossil store address');

    start_cheat_caller_address(contract_address, owner());
    let new_fossil_store = 'NEW_FOSSIL_STORE_ADDRESS'.try_into().unwrap();
    dispatcher.set_fossil_store(new_fossil_store);

    let address_after = dispatcher.get_fossil_store();
    assert(address_after == new_fossil_store, 'invalid fossil store address');
}

#[test]
#[should_panic(expected: 'Caller is not the owner')]
fn test_should_fail_to_set_fossil_store_by_non_owner() {
    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());
    let dispatcher = ISha2InputDispatcher { contract_address };

    // address before
    let address_before = dispatcher.get_fossil_store();
    assert(address_before == fossil_store(), 'invalid fossil store address');

    let new_fossil_store = starknet::contract_address_const::<'NEW_FOSSIL_STORE_ADDRESS'>();
    dispatcher.set_fossil_store(new_fossil_store);
}

#[test]
fn test_should_be_able_to_upgrade_by_owner() {
    let new_class_hash = *declare("TestUpgrade").unwrap().contract_class().class_hash;
    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());

    // // let new_class_hash = get_class_hash(starknet::contract_address_const::<'Sha2InputNew'>());
    // // println!("new_class_hash: {}", new_class_hash);
    let dispatcher = IUpgradeableDispatcher { contract_address };

    start_cheat_caller_address(contract_address, owner());
    dispatcher.upgrade(new_class_hash);

    assert(get_class_hash(contract_address) == new_class_hash, 'did not upgrade correctly');
}

#[test]
#[should_panic(expected: 'Caller is not the owner')]
fn test_should_fail_to_upgrade_by_non_owner() {
    let contract_address = deploy_contract_sha2_input(owner(), fossil_store());

    let new_class_hash = class_hash_const::<'Sha2InputNew'>();
    let dispatcher = IUpgradeableDispatcher { contract_address };
    dispatcher.upgrade(new_class_hash);
}
