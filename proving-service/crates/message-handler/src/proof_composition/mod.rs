#[cfg(feature = "proof-composition")]
use add_twap_7d_error_bound_floating::add_twap_7d_error_bound;
#[cfg(feature = "proof-composition")]
use calculate_pt_pt1_error_bound_floating::calculate_pt_pt1_error_bound_floating;
#[cfg(feature = "proof-composition")]
use coprocessor_common::{
    floating_point,
    original::{self, convert_array1_to_dvec},
    tests::mock::convert_data_to_vec_of_tuples,
};
#[cfg(feature = "proof-composition")]
use coprocessor_core::{
    AddTwap7dErrorBoundFloatingInput, CalculatePtPt1ErrorBoundFloatingInput, HashingFeltInput,
    MaxReturnInput, ProofCompositionInput, RemoveSeasonalityErrorBoundFloatingInput,
    SimulatePriceVerifyPositionInput, TwapErrorBoundInput,
};
use eyre::{Result, eyre};
#[cfg(feature = "proof-composition")]
use hashing_felts::hash_felts;
#[cfg(feature = "proof-composition")]
use max_return_floating::max_return;
#[cfg(feature = "proof-composition")]
use proof_composition_twap_maxreturn_reserveprice_floating_hashing_methods::{
    PROOF_COMPOSITION_TWAP_MAXRETURN_RESERVEPRICE_FLOATING_HASHING_GUEST_ELF,
    PROOF_COMPOSITION_TWAP_MAXRETURN_RESERVEPRICE_FLOATING_HASHING_GUEST_ID,
};
#[cfg(feature = "proof-composition")]
use remove_seasonality_error_bound_floating::remove_seasonality_error_bound;
#[cfg(not(feature = "proof-composition"))]
use risc0_zkvm::Receipt;
#[cfg(feature = "proof-composition")]
use risc0_zkvm::{ExecutorEnv, ProverOpts, Receipt, ReceiptKind, default_prover};
#[cfg(feature = "proof-composition")]
use simulate_price_verify_position_floating::simulate_price_verify_position;
#[cfg(feature = "proof-composition")]
use starknet::core::types::Felt;
#[cfg(feature = "proof-composition")]
use tokio::{task, try_join};
#[cfg(feature = "proof-composition")]
use twap_error_bound_floating::calculate_twap;

#[async_trait::async_trait]
pub trait ProofProvider {
    // TODO: separate composition from generation
    // TODO: add error handling

    async fn generate_proofs_from_data(
        &self,
        start_timestamp: i64,
        end_timestamp: i64,
        raw_input: Vec<String>,
    ) -> Result<Receipt>;
}

#[derive(Debug, Clone)]
pub struct BonsaiProofProvider;

impl BonsaiProofProvider {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for BonsaiProofProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ProofProvider for BonsaiProofProvider {
    #[cfg(feature = "proof-composition")]
    async fn generate_proofs_from_data(
        &self,
        start_timestamp: i64,
        end_timestamp: i64,
        raw_input: Vec<String>,
    ) -> Result<Receipt> {
        use crate::hashing::HashingProvider;

        let hashing_provider = HashingProvider::from_env()?;
        // hashing inputs
        let mut res = Vec::with_capacity(5760);
        for i in 0..5760 {
            let index = i % raw_input.len();
            let felt = Felt::from_hex_unchecked(&raw_input[index]);
            res.push(felt);
        }
        let (hashing_receipt, hashing_res) = hash_felts(HashingFeltInput { inputs: res });

        let data_8_months = hashing_res.f64_inputs;
        let data = data_8_months[data_8_months.len().saturating_sub(2160)..].to_vec();

        // max return
        let input = MaxReturnInput { data: data.clone() };
        let (max_return_receipt, max_return_res) = max_return(input);

        // twap
        // replacing  original::calculate_twap::calculate_twap with this, as we are using random avg fee hourly data
        // that we dont have the underlying raw data for
        let twap_original = floating_point::calculate_twap(&data);
        let input = TwapErrorBoundInput {
            avg_hourly_gas_fee: data.clone(),
            twap_tolerance: 1.0,
            twap_result: twap_original,
        };

        let (calculate_twap_receipt, _calculate_twap_res) = calculate_twap(input);

        // reserve price
        // run rust code in host
        // ensure convergence in host
        let n_periods = 720;

        let data_with_timestamps = convert_data_to_vec_of_tuples(data.clone(), start_timestamp);
        let res = original::calculate_reserve_price(&data_with_timestamps, 15000, n_periods);

        let num_paths = 4000;
        let gradient_tolerance = 5e-2;
        let floating_point_tolerance = 0.00001; // 0.00001%
        let reserve_price_tolerance = 5.0; // 5%

        // Making all these async via tokio spawns

        // Remove seasonality error bound
        let data_clone = data.clone();
        let de_seasonalised_detrended_log_base_fee =
            convert_array1_to_dvec(res.de_seasonalised_detrended_log_base_fee.clone());
        let season_param_clone = convert_array1_to_dvec(res.season_param.clone());
        let slope = res.slope;
        let intercept = res.intercept;

        let remove_seasonality_error_bound_handle = task::spawn_blocking(move || {
            let (receipt, _) =
                remove_seasonality_error_bound(RemoveSeasonalityErrorBoundFloatingInput {
                    data: data_clone,
                    slope,
                    intercept,
                    de_seasonalised_detrended_log_base_fee,
                    season_param: season_param_clone,
                    tolerance: floating_point_tolerance,
                });

            receipt
        });

        // Add twap 7d error bound
        let data_clone = data.clone();
        let twap_7d_clone = res.twap_7d.clone();

        let add_twap_7d_error_bound_handle = task::spawn_blocking(move || {
            let (receipt, _) = add_twap_7d_error_bound(AddTwap7dErrorBoundFloatingInput {
                data: data_clone,
                twap_7d: twap_7d_clone,
                tolerance: floating_point_tolerance,
            });

            receipt
        });

        // Calculate pt pt1 error bound
        let de_seasonalised_detrended_log_base_fee =
            convert_array1_to_dvec(res.de_seasonalised_detrended_log_base_fee.clone());
        let pt = convert_array1_to_dvec(res.pt.clone());
        let pt_1 = convert_array1_to_dvec(res.pt_1.clone());

        let calculate_pt_pt1_error_bound_handle = task::spawn_blocking(move || {
            let (receipt, _) =
                calculate_pt_pt1_error_bound_floating(CalculatePtPt1ErrorBoundFloatingInput {
                    de_seasonalised_detrended_log_base_fee,
                    pt,
                    pt_1,
                    tolerance: floating_point_tolerance,
                });

            receipt
        });

        // Simulate price verify position
        let data_length = data.len();
        let positions = res.positions.clone();
        let de_seasonalised_detrended_log_base_fee =
            convert_array1_to_dvec(res.de_seasonalised_detrended_log_base_fee.clone());
        let pt = convert_array1_to_dvec(res.pt.clone());
        let pt_1 = convert_array1_to_dvec(res.pt_1.clone());
        let season_param = convert_array1_to_dvec(res.season_param.clone());
        let twap_7d = res.twap_7d.clone();
        let slope = res.slope;
        let intercept = res.intercept;
        let reserve_price = res.reserve_price;

        let simulate_price_verify_position_handle = task::spawn_blocking(move || {
            let (receipt, _) = simulate_price_verify_position(SimulatePriceVerifyPositionInput {
                start_timestamp,
                end_timestamp,
                data_length,
                positions,
                pt,
                pt_1,
                gradient_tolerance,
                de_seasonalised_detrended_log_base_fee,
                n_periods,
                num_paths,
                season_param,
                twap_7d,
                slope,
                intercept,
                reserve_price,
                tolerance: reserve_price_tolerance, // 5%
            });

            receipt
        });

        // Make composite proof

        let input = ProofCompositionInput {
            data_8_months_hash: hashing_res.hash,
            data_8_months,
            start_timestamp,
            end_timestamp,
            positions: res.positions,
            pt: convert_array1_to_dvec(res.pt),
            pt_1: convert_array1_to_dvec(res.pt_1),
            gradient_tolerance,
            de_seasonalised_detrended_log_base_fee: convert_array1_to_dvec(
                res.de_seasonalised_detrended_log_base_fee,
            ),
            n_periods,
            num_paths,
            season_param: convert_array1_to_dvec(res.season_param),
            twap_7d: res.twap_7d,
            slope: res.slope,
            intercept: res.intercept,
            reserve_price: res.reserve_price,
            floating_point_tolerance,
            reserve_price_tolerance,
            twap_result: twap_original,
            twap_tolerance: 1.0,
            max_return: max_return_res.1,
        };

        // try to join for all async tasks
        let result_receipt = try_join!(
            remove_seasonality_error_bound_handle,
            add_twap_7d_error_bound_handle,
            calculate_pt_pt1_error_bound_handle,
            simulate_price_verify_position_handle
        );

        let result_receipt = match result_receipt {
            Ok(receipts) => receipts,
            Err(e) => {
                return Err(eyre!("Failed to join tasks: {}", e));
            }
        };

        // Composite proof generation
        let env = ExecutorEnv::builder()
            .add_assumption(hashing_receipt)
            .add_assumption(calculate_twap_receipt)
            .add_assumption(max_return_receipt)
            .add_assumption(result_receipt.0)
            .add_assumption(result_receipt.1)
            .add_assumption(result_receipt.2)
            .add_assumption(result_receipt.3)
            .write(&input)
            .map_err(|e| eyre!("Failed to write input to executor: {}", e))?
            .build()
            .map_err(|e| eyre!("Failed to build executor environment: {}", e))?;

        let prover_opts = ProverOpts::default().with_receipt_kind(ReceiptKind::Groth16);

        let prove_info = default_prover()
            .prove_with_opts(
                env,
                PROOF_COMPOSITION_TWAP_MAXRETURN_RESERVEPRICE_FLOATING_HASHING_GUEST_ELF,
                &prover_opts,
            )
            .map_err(|e| eyre!("Failed to prove: {}", e))?;

        let receipt = prove_info.receipt;
        receipt
            .verify(PROOF_COMPOSITION_TWAP_MAXRETURN_RESERVEPRICE_FLOATING_HASHING_GUEST_ID)
            .map_err(|e| eyre!("Failed to verify proof: {}", e))?;

        Ok(receipt)
    }

    #[cfg(not(feature = "proof-composition"))]
    async fn generate_proofs_from_data(
        &self,
        _start_timestamp: i64,
        _end_timestamp: i64,
        _raw_input: Vec<String>,
    ) -> Result<Receipt> {
        Err(eyre!(
            "Proof composition is disabled. Enable the 'proof-composition' feature to use this functionality."
        ))
    }
}
