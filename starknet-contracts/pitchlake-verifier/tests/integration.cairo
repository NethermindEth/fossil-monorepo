use pitchlake_verifier::PitchLakeJobRequest;
use pitchlake_verifier::fixtures::PROOF_CALLDATA;
use pitchlake_verifier::pitchlake_verifier::{
    IPitchLakeVerifierDispatcher, IPitchLakeVerifierDispatcherTrait,
};
use snforge_std::{ContractClassTrait, DeclareResultTrait, declare};

fn OWNER() -> starknet::ContractAddress {
    'OWNER'.try_into().unwrap()
}

fn deploy() -> IPitchLakeVerifierDispatcher {
    let ecip_class = declare("UniversalECIP").unwrap().contract_class();
    let ecip_class_felt: felt252 = (*ecip_class.class_hash).into();

    let risc0_class = declare("Risc0Groth16VerifierBN254").unwrap().contract_class();
    let (risc0_verifier_address, _) = risc0_class.deploy(@array![ecip_class_felt]).unwrap();

    let fossil_client_class = declare("MockPitchLakeClient").unwrap().contract_class();
    let (fossil_client_address, _) = fossil_client_class.deploy(@array![]).unwrap();

    let pl_verifier_class = declare("PitchLakeVerifier").unwrap().contract_class();
    let (pl_verifier_address, _) = pl_verifier_class
        .deploy(
            @array![risc0_verifier_address.into(), fossil_client_address.into(), OWNER().into()],
        )
        .unwrap();

    IPitchLakeVerifierDispatcher { contract_address: pl_verifier_address }
}

#[test]
fn test_deploy() {
    let _pl_verifier = deploy();
}

#[test]
fn test_verify() {
    let pl_verifier = deploy();

    let job_request = PitchLakeJobRequest {
        vault_address: 'VAULT_ADDRESS'.try_into().unwrap(),
        timestamp: 1234567890,
        program_id: 'PROGRAM_ID',
    };
    let proof_calldata = PROOF_CALLDATA().span();

    let _ = pl_verifier.verify_proof(proof_calldata, job_request);
}
