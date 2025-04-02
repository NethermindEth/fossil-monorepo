pub mod docker;
pub use docker::*;

pub mod mock_fossil_light_client;
pub use mock_fossil_light_client::*;

pub const LOCALHOST_RPC_URL: &str = "http://localhost:5050";
pub const LOCALHOST_STARKNET_PRIVATE_KEY: &str = "0x0000000000000000000000000000000071d7bb07b9a64f6f78ac4c816aff4da9";
pub const LOCALHOST_STARKNET_ACCOUNT_ADDRESS: &str = "0x064b48806902a367c8598f4f95c305e8c1a1acba5f082d294a43793113115691";
pub const LOCALHOST_FOSSIL_LIGHT_CLIENT_ADDRESS: &str = "0x02b27ec0fef7b477caf663469037147112384d27ed74b6d4dd647f47965f1884";
pub const LOCALHOST_HASH_STORAGE_ADDRESS: &str = "0x05b9f7e2a0903515dfab2805df42e651a5c56222d83c5361d7a9e332e114229a";


// this works
// starkli invoke 0x05b9f7e2a0903515dfab2805df42e651a5c56222d83c5361d7a9e332e114229a hash_avg_fees_and_store 1 --rpc http://localhost:5050 --private-key 0x0000000000000000000000000000000071d7bb07b9a64f6f78ac4c816aff4da9 --account account.json    


// starkli invoke 0x05b9f7e2a0903515dfab2805df42e651a5c56222d83c5361d7a9e332e114229a hash_avg_fees_and_store 1 --rpc http://starknet-devnet:5050 --private-key 0x0000000000000000000000000000000071d7bb07b9a64f6f78ac4c816aff4da9 --account account.json 