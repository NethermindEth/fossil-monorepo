pub mod helper;
pub mod interface;
pub mod mock;

#[starknet::interface]
pub trait ISha2Input<TContractState> {
    fn set_fossil_store(ref self: TContractState, fossil_store: starknet::ContractAddress);
    fn get_fossil_store(self: @TContractState) -> starknet::ContractAddress;
    fn hash_avg_fees_and_store(ref self: TContractState, start_timestamp: u64);
    fn get_hash_stored_avg_fees(self: @TContractState, timestamp: u64) -> [u32; 8];
    fn hash_batched_avg_fees(ref self: TContractState, start_timestamp: u64);
    fn get_hash_stored_batched_avg_fees(self: @TContractState, timestamp: u64) -> [u32; 8];
}

#[starknet::contract]
mod Sha2Input {
    use core::sha256::compute_sha256_u32_array;
    use openzeppelin_access::ownable::OwnableComponent;
    use openzeppelin_upgrades::UpgradeableComponent;
    use openzeppelin_upgrades::interface::IUpgradeable;
    use starknet::storage::{
        Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess,
    };
    use super::helper::Helper::convert_avg_fees_to_u32_array;
    use super::interface::fossil_store::{
        IFossilMinimalAvgFeeStoreDispatcher, IFossilMinimalAvgFeeStoreDispatcherTrait,
    };

    component!(path: OwnableComponent, storage: ownable, event: OwnableEvent);
    component!(path: UpgradeableComponent, storage: upgradeable, event: UpgradeableEvent);

    #[storage]
    struct Storage {
        fossil_store: IFossilMinimalAvgFeeStoreDispatcher,
        hash_stored_avg_fees: Map<u64, [u32; 8]>, // hash of 180 avg fees
        hash_batched_avg_fees: Map<u64, [u32; 8]>, // hash of hash of 180 avg fees
        #[substorage(v0)]
        ownable: OwnableComponent::Storage,
        #[substorage(v0)]
        upgradeable: UpgradeableComponent::Storage,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        HashStoredAvgFees: HashStoredAvgFees,
        HashStoredBatchedAvgFees: HashStoredBatchedAvgFees,
        #[flat]
        OwnableEvent: OwnableComponent::Event,
        #[flat]
        UpgradeableEvent: UpgradeableComponent::Event,
    }

    #[derive(Drop, starknet::Event)]
    struct HashStoredAvgFees {
        timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    struct HashStoredBatchedAvgFees {
        timestamp: u64,
    }

    #[abi(embed_v0)]
    impl OwnableMixinImpl = OwnableComponent::OwnableMixinImpl<ContractState>;

    // Internal
    impl OwnableInternalImpl = OwnableComponent::InternalImpl<ContractState>;
    impl UpgradeableInternalImpl = UpgradeableComponent::InternalImpl<ContractState>;


    #[constructor]
    fn constructor(
        ref self: ContractState,
        owner: starknet::ContractAddress,
        fossil_store: starknet::ContractAddress,
    ) {
        self.set_fossil_store(fossil_store);
        self.ownable.initializer(owner);
    }

    #[abi(embed_v0)]
    impl Sha2InputImpl of super::ISha2Input<ContractState> {
        fn set_fossil_store(ref self: ContractState, fossil_store: starknet::ContractAddress) {
            self.ownable.assert_only_owner();
            self
                .fossil_store
                .write(IFossilMinimalAvgFeeStoreDispatcher { contract_address: fossil_store });
        }

        fn get_fossil_store(self: @ContractState) -> starknet::ContractAddress {
            self.fossil_store.read().contract_address
        }

        // hashing 180 avg fees
        fn hash_avg_fees_and_store(ref self: ContractState, start_timestamp: u64) {
            let mut result_array = array![];
            let fossil_store = self.fossil_store.read();
            for i in 0..180_u64 { // hashing of 180 avg fees
                let timestamp = start_timestamp + (i * 3600_u64);
                // TODO: check if get_avg_fee returns average that has been fully calculated (ie.
                // after weighted mean)
                // pending changes in fossil-light-client
                let avg_fees = fossil_store.get_avg_fee(timestamp);
                assert(avg_fees != 0, 'Avg fees is 0');
                let avg_fees_array = convert_avg_fees_to_u32_array(avg_fees);
                result_array.append_span(avg_fees_array.span());
            }

            let hash_res = compute_sha256_u32_array(result_array, 0, 0);
            self.hash_stored_avg_fees.entry(start_timestamp).write(hash_res);

            self.emit(HashStoredAvgFees { timestamp: start_timestamp });
        }

        fn get_hash_stored_avg_fees(self: @ContractState, timestamp: u64) -> [u32; 8] {
            self.hash_stored_avg_fees.entry(timestamp).read()
        }

        fn hash_batched_avg_fees(ref self: ContractState, start_timestamp: u64) {
            let mut result_array = array![];
            let num_in_a_batch = 32_u64; // 5760/180
            for i in 0..num_in_a_batch {
                let timestamp = start_timestamp + (i * 3600_u64 * 180_u64);
                let hash_avg_fees = self.get_hash_stored_avg_fees(timestamp);

                let is_hash_empty = self.check_hash_is_empty(hash_avg_fees);
                assert(!is_hash_empty, 'Hash is empty');
                result_array.append_span(hash_avg_fees.span());
            }

            let hash_res = compute_sha256_u32_array(result_array, 0, 0);
            self.hash_batched_avg_fees.entry(start_timestamp).write(hash_res);

            self.emit(HashStoredBatchedAvgFees { timestamp: start_timestamp });
        }

        fn get_hash_stored_batched_avg_fees(self: @ContractState, timestamp: u64) -> [u32; 8] {
            self.hash_batched_avg_fees.entry(timestamp).read()
        }
    }

    #[abi(embed_v0)]
    impl UpgradeableImpl of IUpgradeable<ContractState> {
        fn upgrade(ref self: ContractState, new_class_hash: starknet::ClassHash) {
            self.ownable.assert_only_owner();
            self.upgradeable.upgrade(new_class_hash);
        }
    }

    #[generate_trait]
    impl Sha2InputInternalImpl of Sha2InputInternalTrait {
        fn check_hash_is_empty(self: @ContractState, hash: [u32; 8]) -> bool {
            let mut is_empty = true;
            let hash_span = hash.span();
            for i in 0..8_u32 {
                if *hash_span[i] != 0_u32 {
                    is_empty = false;
                }
            }
            is_empty
        }
    }
}
