use derive_more::From;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;

use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use thiserror::Error;

use crate::oracle_config::PoolContractParameters;

#[derive(Clone)]
pub struct PoolContract {
    ergo_tree: ErgoTree,
    refresh_nft_index: usize,
    update_nft_index: usize,
}

#[derive(Debug, From, Error)]
pub enum PoolContractError {
    #[error("pool contract: failed to get update NFT from constants")]
    NoUpdateNftId,
    #[error("pool contract: failed to get refresh NFT from constants")]
    NoRefreshNftId,
    #[error("pool contract: unknown refresh NFT in box")]
    UnknownRefreshNftId,
    #[error("pool contract: unknown update NFT in box")]
    UnknownUpdateNftId,
    #[error("pool contract: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("pool contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("pool contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
}

impl PoolContract {
    pub fn new(parameters: &PoolContractParameters) -> Result<Self, PoolContractError> {
        let ergo_tree = parameters
            .p2s
            .address()
            .script()?
            .with_constant(
                parameters.refresh_nft_index,
                parameters.refresh_nft_token_id.clone().into(),
            )?
            .with_constant(
                parameters.update_nft_index,
                parameters.update_nft_token_id.clone().into(),
            )?;
        let contract = Self::from_ergo_tree(ergo_tree, parameters)?;
        Ok(contract)
    }

    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        parameters: &PoolContractParameters,
    ) -> Result<Self, PoolContractError> {
        dbg!(ergo_tree.get_constants().unwrap());
        let token_id = ergo_tree
            .get_constant(parameters.refresh_nft_index)
            .map_err(|_| PoolContractError::NoRefreshNftId)?
            .ok_or(PoolContractError::NoRefreshNftId)?
            .try_extract_into::<TokenId>();
        match token_id {
            Ok(token_id) => {
                if token_id != parameters.refresh_nft_token_id {
                    return Err(PoolContractError::UnknownRefreshNftId);
                }
            }
            Err(e) => {
                return Err(PoolContractError::TryExtractFrom(e));
            }
        };

        let token_id = ergo_tree
            .get_constant(parameters.update_nft_index)
            .map_err(|_| PoolContractError::NoUpdateNftId)?
            .ok_or(PoolContractError::NoUpdateNftId)?
            .try_extract_into::<TokenId>();
        match token_id {
            Ok(token_id) => {
                if token_id != parameters.update_nft_token_id {
                    return Err(PoolContractError::UnknownUpdateNftId);
                }
            }
            Err(e) => {
                return Err(PoolContractError::TryExtractFrom(e));
            }
        };
        Ok(Self {
            ergo_tree,
            refresh_nft_index: parameters.refresh_nft_index,
            update_nft_index: parameters.update_nft_index,
        })
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn refresh_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.refresh_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn update_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.update_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::pool_commands::test_utils::make_pool_contract_parameters;

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let parameters = make_pool_contract_parameters();
        let refresh_nft_token_id = parameters.refresh_nft_token_id.clone();
        let update_nft_token_id = parameters.update_nft_token_id.clone();
        let c = PoolContract::new(&parameters).unwrap();
        assert_eq!(c.refresh_nft_token_id(), refresh_nft_token_id,);
        assert_eq!(c.update_nft_token_id(), update_nft_token_id,);
    }
}
