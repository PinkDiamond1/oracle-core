use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use thiserror::Error;

use crate::contracts::pool::PoolContract;
use crate::contracts::pool::PoolContractError;
use crate::contracts::pool::PoolContractInputs;
use crate::contracts::pool::PoolContractParameters;

pub trait PoolBox {
    fn contract(&self) -> &PoolContract;
    fn pool_nft_token(&self) -> Token;
    fn reward_token(&self) -> Token;
    fn epoch_counter(&self) -> u32;
    fn rate(&self) -> i64;
    fn get_box(&self) -> &ErgoBox;
}

#[derive(Debug, Error)]
pub enum PoolBoxError {
    #[error("pool box: no tokens found")]
    NoTokens,
    #[error("pool box: no data point in R4")]
    NoDataPoint,
    #[error("pool box: no epoch counter in R5")]
    NoEpochCounter,
    #[error("pool box: no reward token found")]
    NoRewardToken,
    #[error("pool box: {0:?}")]
    PoolContractError(#[from] PoolContractError),
    #[error("pool box: unknown pool NFT token id in box")]
    UnknownPoolNftId,
    #[error("pool box: unknown reward token id in box")]
    UnknownRewardTokenId,
}

#[derive(Clone, Debug)]
pub struct PoolBoxWrapper {
    ergo_box: ErgoBox,
    contract: PoolContract,
}

impl PoolBoxWrapper {
    pub fn new(b: ErgoBox, inputs: &PoolBoxWrapperInputs) -> Result<Self, PoolBoxError> {
        if let Some(token) = b.tokens.as_ref().ok_or(PoolBoxError::NoTokens)?.get(0) {
            if token.token_id != inputs.pool_nft_token_id {
                return Err(PoolBoxError::UnknownPoolNftId);
            }
        } else {
            return Err(PoolBoxError::NoTokens);
        }

        // No need to analyse the data point as its validity is checked within the refresh contract.
        if b.get_register(NonMandatoryRegisterId::R4.into())
            .ok_or(PoolBoxError::NoDataPoint)?
            .try_extract_into::<i64>()
            .is_err()
        {
            return Err(PoolBoxError::NoDataPoint);
        }

        // No need to analyse the epoch counter as its validity is checked within the pool and
        // oracle contracts.
        if b.get_register(NonMandatoryRegisterId::R5.into())
            .ok_or(PoolBoxError::NoEpochCounter)?
            .try_extract_into::<i32>()
            .is_err()
        {
            return Err(PoolBoxError::NoEpochCounter);
        }

        if let Some(reward_token) = b.tokens.as_ref().ok_or(PoolBoxError::NoTokens)?.get(1) {
            if reward_token.token_id != inputs.reward_token_id {
                return Err(PoolBoxError::UnknownRewardTokenId);
            }
        } else {
            return Err(PoolBoxError::NoRewardToken);
        }
        let contract = PoolContract::from_ergo_tree(b.ergo_tree.clone(), &inputs.contract_inputs)?;
        Ok(Self {
            ergo_box: b,
            contract,
        })
    }
}

impl PoolBox for PoolBoxWrapper {
    fn pool_nft_token(&self) -> Token {
        self.ergo_box
            .tokens
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap()
            .clone()
    }

    fn epoch_counter(&self) -> u32 {
        self.ergo_box
            .get_register(NonMandatoryRegisterId::R5.into())
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    fn rate(&self) -> i64 {
        self.ergo_box
            .get_register(NonMandatoryRegisterId::R4.into())
            .unwrap()
            .try_extract_into::<i64>()
            .unwrap()
    }

    fn reward_token(&self) -> Token {
        self.ergo_box
            .tokens
            .as_ref()
            .unwrap()
            .get(1)
            .unwrap()
            .clone()
    }

    fn get_box(&self) -> &ErgoBox {
        &self.ergo_box
    }

    fn contract(&self) -> &PoolContract {
        &self.contract
    }
}

#[derive(Clone, Debug)]
pub struct PoolBoxWrapperInputs {
    pub contract_inputs: PoolContractInputs,
    /// Pool NFT token is expected to reside in `tokens(0)` of the pool box.
    pub pool_nft_token_id: TokenId,
    /// Reward token is expected to reside in `tokens(1)` of the pool box.
    pub reward_token_id: TokenId,
}

impl PoolBoxWrapperInputs {
    pub fn build_with(
        contract_parameters: PoolContractParameters,
        refresh_nft_token_id: TokenId,
        update_nft_token_id: TokenId,
        pool_nft_token_id: TokenId,
        reward_token_id: TokenId,
    ) -> Result<Self, PoolContractError> {
        let contract_inputs = PoolContractInputs::build_with(
            contract_parameters,
            refresh_nft_token_id,
            update_nft_token_id,
        )?;
        Ok(Self {
            contract_inputs,
            pool_nft_token_id,
            reward_token_id,
        })
    }

    pub fn checked_load(
        contract_parameters: PoolContractParameters,
        refresh_nft_token_id: TokenId,
        update_nft_token_id: TokenId,
        pool_nft_token_id: TokenId,
        reward_token_id: TokenId,
    ) -> Result<Self, PoolContractError> {
        let contract_inputs = PoolContractInputs::checked_load(
            contract_parameters,
            refresh_nft_token_id,
            update_nft_token_id,
        )?;
        Ok(Self {
            contract_inputs,
            pool_nft_token_id,
            reward_token_id,
        })
    }
}

pub fn make_pool_box_candidate(
    contract: &PoolContract,
    datapoint: i64,
    epoch_counter: i32,
    pool_nft_token: Token,
    reward_token: Token,
    value: BoxValue,
    creation_height: u32,
) -> Result<ErgoBoxCandidate, ErgoBoxCandidateBuilderError> {
    let mut builder = ErgoBoxCandidateBuilder::new(value, contract.ergo_tree(), creation_height);
    builder.set_register_value(NonMandatoryRegisterId::R4, datapoint.into());
    builder.set_register_value(NonMandatoryRegisterId::R5, epoch_counter.into());
    builder.add_token(pool_nft_token.clone());
    builder.add_token(reward_token.clone());
    builder.build()
}
