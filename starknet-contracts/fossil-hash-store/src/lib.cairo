// FOSSIL HASH STORE CONTRACT
//
// This contract provides a two-tier cryptographic hashing system for batch fee data verification.
// It enables efficient onchain proof verification by pre-computing and storing hashes of historical
// fee data from the Fossil Store contract.
//
// ARCHITECTURE:
// 1. Tier 1 - Batch Hashes (hash_stored_avg_fees):
//    - Each batch represents 180 consecutive hourly fee values (7.5 days)
//    - Hashed using SHA256 and stored by start_timestamp
//    - Created via: hash_avg_fees_and_store(start_timestamp)
//
// 2. Tier 2 - Composite Hash (hash_batched_avg_fees):
//    - Combines multiple Tier 1 batch hashes into a single composite hash
//    - Production: 32 batches (5760 hours = 8 months of data)
//    - Current configuration: 8 batches (1440 hours = 2 months of data)
//    - Created via: hash_batched_avg_fees(start_timestamp)
//
// PROOF VERIFICATION WORKFLOW:
// 1. Message handler determines required timestamp range for proof
// 2. Calls hash_avg_fees_and_store() for each 180-hour batch that doesn't exist yet
// 3. Calls hash_batched_avg_fees() to create composite hash from all batch hashes
// 4. Composite hash is used in RISC0 proof verification to validate data integrity
//
// CONFIGURATION:
// - Batch size: 180 hours (fixed, matches RISC0 guest program expectations)
// - Number of batches: Configurable via num_in_a_batch in hash_batched_avg_fees()
//   - Production: 32 batches for 8-month historical data analysis
//   - Current: 8 batches (see line 130 for configuration)
//
// DEPENDENCIES:
// - Fossil Store contract: Source of truth for hourly average fee data
// - Must be set via set_fossil_store() or constructor

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
        self.ownable.initializer(owner);
        self
            .fossil_store
            .write(IFossilMinimalAvgFeeStoreDispatcher { contract_address: fossil_store });
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

        /// Creates and stores a SHA256 hash of 180 consecutive hourly average fees.
        ///
        /// This is Tier 1 of the two-tier hashing system. Each invocation processes
        /// 180 hours (7.5 days) of fee data starting from start_timestamp.
        ///
        /// Arguments:
        /// - start_timestamp: Must be normalized to hour boundary (divisible by 3600)
        ///                   Represents the first hour in the 180-hour batch
        ///
        /// Fee Data Requirements:
        /// - All 180 hourly fee values must be available in the Fossil Store
        /// - The Fossil Store has an indexing lag (currently ~12 hours)
        /// - Callers should ensure start_timestamp accounts for this lag
        ///
        /// Hash Computation:
        /// - Fetches fees for timestamps: start_timestamp + (i * 3600) where i ∈ [0, 180)
        /// - Each fee is converted to a u32 array representation
        /// - All fee arrays are concatenated and hashed with SHA256
        /// - Result is an 8-element u32 array representing the 256-bit hash
        fn hash_avg_fees_and_store(ref self: ContractState, start_timestamp: u64) {
            let mut result_array = array![];
            let fossil_store = self.fossil_store.read();

            // Process 180 consecutive hourly fee values (7.5 days)
            for i in 0..180_u64 {
                let timestamp = start_timestamp + (i * 3600_u64);

                // DEVELOPER NOTE: Fossil Store Data Validation
                // The get_avg_fee() call retrieves the weighted mean average fee for the given hour.
                // The Fossil Store computes this via the fossil-light-client indexer.
                //
                // DATA INTEGRITY CONSIDERATION:
                // In production, consider validating that avg_fees != 0 to catch missing data:
                //   assert(avg_fees != 0, 'Avg fees is 0');
                //
                // This assertion is currently disabled to allow flexibility with historical data
                // availability. Re-enable if strict data completeness is required.
                let avg_fees = fossil_store.get_avg_fee(timestamp);

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

        /// Creates and stores a composite SHA256 hash from multiple batch hashes.
        ///
        /// This is Tier 2 of the two-tier hashing system. It combines multiple Tier 1
        /// batch hashes into a single composite hash for efficient proof verification.
        ///
        /// Arguments:
        /// - start_timestamp: Must be normalized to hour boundary (divisible by 3600)
        ///                   Represents the first hour of the first batch
        ///
        /// Prerequisites:
        /// - All required Tier 1 batch hashes must exist (created via hash_avg_fees_and_store)
        /// - For num_in_a_batch = N, requires N batch hashes at intervals of 180 hours
        /// - Example: If start_timestamp = T, requires batch hashes at:
        ///   T, T+180h, T+360h, ..., T+((N-1)*180)h
        ///
        /// Configuration:
        /// - num_in_a_batch: Number of 180-hour batches to combine
        ///   - Current: 8 batches = 1440 hours (2 months of data)
        ///   - Production: 32 batches = 5760 hours (8 months of data)
        ///   - This value must match the RISC0 guest program's expected data length
        ///   - Changing this requires coordination with the message-handler proof generation logic
        ///
        /// Error Handling:
        /// - Panics with 'Hash is empty' if any required batch hash doesn't exist
        /// - Ensures data integrity by validating all batch hashes before composition
        fn hash_batched_avg_fees(ref self: ContractState, start_timestamp: u64) {
            let mut result_array = array![];

            // CONFIGURATION: Number of batches to combine
            // ============================================
            // Each batch represents 180 hours of fee data.
            //
            // Production configuration (8 months of historical data):
            //   let num_in_a_batch = 32_u64; // 32 * 180 hours = 5760 hours = 8 months
            //
            // Current configuration (reduced data requirement):
            let num_in_a_batch = 8_u64; // 8 * 180 hours = 1440 hours = 2 months
            //
            // IMPORTANT: When changing num_in_a_batch, also update:
            // 1. Message handler: proof_composition/mod.rs (REQUIRED_HOURS constant)
            // 2. RISC0 guest program: Ensure it expects the correct data length

            for i in 0..num_in_a_batch {
                // Each batch starts 180 hours after the previous one
                let timestamp = start_timestamp + (i * 3600_u64 * 180_u64);
                let hash_avg_fees = self.get_hash_stored_avg_fees(timestamp);

                // Validate that the batch hash exists (non-zero)
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
