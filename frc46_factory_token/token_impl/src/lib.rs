use cid::{multihash::Code, Cid};
use frc42_dispatch::match_method;
use frc46_token::token::{
    state::{StateError, TokenState},
    types::{
        AllowanceReturn, BalanceReturn, BurnFromReturn, BurnParams, BurnReturn,
        DecreaseAllowanceParams, FRC46Token, GetAllowanceParams, GranularityReturn,
        IncreaseAllowanceParams, MintReturn, RevokeAllowanceParams, TotalSupplyReturn,
        TransferFromParams, TransferFromReturn, TransferParams, TransferReturn,
    },
    Token, TokenError,
};
use fvm_actor_utils::{
    blockstore::Blockstore as RuntimeBlockstore,
    messaging::MessagingError,
    receiver::ReceiverHookError,
    syscalls::{fvm_syscalls::FvmSyscalls, NoStateError, Syscalls},
    util::ActorRuntime,
};
use fvm_ipld_blockstore::{Block, Blockstore};
use fvm_ipld_encoding::{
    tuple::{Deserialize_tuple, Serialize_tuple},
    CborStore, RawBytes, DAG_CBOR,
};
<<<<<<< HEAD
use fvm_sdk::error::{StateReadError, StateUpdateError};
=======
>>>>>>> 8b45b3e (Add set_root to ActorRuntime)
use fvm_sdk::{self as sdk, sys::ErrorNumber, NO_DATA_BLOCK_ID};
use fvm_shared::{address::Address, econ::TokenAmount, error::ExitCode, ActorID};
use serde::{de::DeserializeOwned, ser::Serialize};
use thiserror::Error;

/// Errors that can occur during the execution of this actor
#[derive(Error, Debug)]
pub enum RuntimeError {
    /// Error from the underlying token library
    #[error("error in token: {0}")]
    Token(#[from] TokenError),
    /// Error from the underlying universal receiver hook library
    #[error("error calling receiver hook: {0}")]
    Receiver(#[from] ReceiverHookError),
    /// Error from serialising data to RawBytes
    #[error("ipld encoding error: {0}")]
    Encoding(#[from] fvm_ipld_encoding::Error),
    #[error("ipld blockstore error: {0}")]
    Blockstore(#[from] ErrorNumber),
    #[error("actor state not found {0}")]
    NoState(#[from] StateReadError),
    #[error("failed to update actor state {0}")]
    StateUpdate(#[from] StateUpdateError),
    // deserialisation error when loading state
    #[error("error loading state {0}")]
    Deserialization(String),
    // serialisation error when saving state
    #[error("error saving state {0}")]
    Serialization(String),
    #[error("underlying state error {0}")]
    State(#[from] StateError),
    #[error("actor messaging error {0}")]
    Messaging(#[from] MessagingError),
    #[error("address not authorized")]
    AddressNotAuthorized,
    #[error("minting has been permanently disabled")]
    MintingDisabled,
}

impl From<sdk::error::NoStateError> for RuntimeError {
    fn from(_: sdk::error::NoStateError) -> Self {
        RuntimeError::NoState(NoStateError)
    }
}

impl From<&RuntimeError> for ExitCode {
    fn from(error: &RuntimeError) -> Self {
        match error {
            RuntimeError::Token(e) => e.into(),
            RuntimeError::Receiver(e) => e.into(),
            RuntimeError::Encoding(_) => ExitCode::USR_SERIALIZATION,
            RuntimeError::Blockstore(e) => match e {
                ErrorNumber::IllegalArgument => ExitCode::USR_ILLEGAL_ARGUMENT,
                ErrorNumber::Forbidden | ErrorNumber::IllegalOperation => ExitCode::USR_FORBIDDEN,
                ErrorNumber::AssertionFailed => ExitCode::USR_ASSERTION_FAILED,
                ErrorNumber::InsufficientFunds => ExitCode::USR_INSUFFICIENT_FUNDS,
                ErrorNumber::IllegalCid | ErrorNumber::NotFound | ErrorNumber::InvalidHandle => {
                    ExitCode::USR_NOT_FOUND
                }
                ErrorNumber::Serialization | ErrorNumber::IllegalCodec => {
                    ExitCode::USR_SERIALIZATION
                }
                _ => ExitCode::USR_UNSPECIFIED,
            },
            RuntimeError::NoState(_) => ExitCode::USR_NOT_FOUND,
            RuntimeError::StateUpdate(e) => match e {
                StateUpdateError::ActorDeleted => ExitCode::USR_ILLEGAL_STATE,
                StateUpdateError::ReadOnly => ExitCode::USR_READ_ONLY,
            },
            // RuntimeError::StateUpdate(_) => ExitCode::USR_ILLEGAL_STATE,
            RuntimeError::Deserialization(_) | RuntimeError::Serialization(_) => {
                ExitCode::USR_SERIALIZATION
            }
            RuntimeError::State(e) => e.into(),
            RuntimeError::Messaging(e) => e.into(),
            RuntimeError::AddressNotAuthorized | RuntimeError::MintingDisabled => {
                ExitCode::USR_FORBIDDEN
            }
        }
    }
}

#[derive(Serialize_tuple, Deserialize_tuple, Debug)]
pub struct ConstructorParams {
    pub name: String,
    pub symbol: String,
    pub granularity: u64,
    /// authorised mint operator
    /// only this address can mint tokens or remove themselves to permanently disable minting
    pub minter: Address,
}

pub fn construct_token(params: ConstructorParams) -> Result<u32, RuntimeError> {
    let runtime = ActorRuntime::<FvmSyscalls, RuntimeBlockstore>::new_fvm_runtime();
    let minter = runtime.resolve_id(&params.minter)?;
    let token =
        FactoryToken::new(runtime, params.name, params.symbol, params.granularity, Some(minter));

    let cid = token.save()?;
    fvm_sdk::sself::set_root(&cid).map_err(|_| NoStateError)?;

    Ok(NO_DATA_BLOCK_ID)
}

#[derive(Serialize_tuple, Deserialize_tuple, Debug)]
pub struct FactoryTokenState {
    /// Default token helper impl
    pub token: TokenState,
    /// basic token identifier stuff, should it go here or store separately alongside the state
    pub name: String,
    pub symbol: String,
    pub granularity: u64,
    /// address of authorised minting operator
    pub minter: Option<ActorID>,
}

pub struct FactoryToken<S: Syscalls + Clone, BS: Blockstore + Clone> {
    runtime: ActorRuntime<S, BS>,
    state: FactoryTokenState,
}

impl FactoryTokenState {
    /// Load token state from the blockstore provided in `runtime`
    /// This is for internal use only as part of FactoryToken::load
    fn load<BS: Blockstore>(runtime: &BS, cid: &Cid) -> Result<Self, RuntimeError> {
        match runtime.get_cbor::<Self>(cid) {
            Ok(Some(s)) => Ok(s),
            // TODO: improve on these errors?
            Ok(None) => Err(RuntimeError::Deserialization("no data found".into())),
            Err(e) => Err(RuntimeError::Deserialization(e.to_string())),
        }
    }
}

/// Implementation of the token API in a FVM actor
///
/// Here the Ipld parameter structs are marshalled and passed to the underlying library functions
impl<SC: Syscalls + Clone, BS: Blockstore + Clone> FRC46Token for FactoryToken<SC, BS> {
    type TokenError = RuntimeError;
    fn name(&self) -> String {
        self.state.name.clone()
    }

    fn symbol(&self) -> String {
        self.state.symbol.clone()
    }

    fn granularity(&self) -> GranularityReturn {
        self.state.granularity
    }

    fn total_supply(&mut self) -> TotalSupplyReturn {
        self.token().total_supply()
    }

    fn balance_of(&mut self, params: Address) -> Result<BalanceReturn, RuntimeError> {
        Ok(self.token().balance_of(&params)?)
    }

    fn transfer(&mut self, params: TransferParams) -> Result<TransferReturn, RuntimeError> {
        let operator = self.caller_address();
        let mut hook = self.token().transfer(
            &operator,
            &params.to,
            &params.amount,
            params.operator_data,
            RawBytes::default(),
        )?;

        let cid = self.save()?;
        self.runtime.set_root(&cid).map_err(|_| NoStateError)?;

        let hook_ret = hook.call(self.token().runtime())?;

        self.reload(&cid)?;
        let ret = self.token().transfer_return(hook_ret)?;

        Ok(ret)
    }

    fn transfer_from(
        &mut self,
        params: TransferFromParams,
    ) -> Result<TransferFromReturn, RuntimeError> {
        let operator = self.caller_address();
        let mut hook = self.token().transfer_from(
            &operator,
            &params.from,
            &params.to,
            &params.amount,
            params.operator_data,
            RawBytes::default(),
        )?;

        let cid = self.save()?;
        self.runtime.set_root(&cid).map_err(|_| NoStateError)?;

        let hook_ret = hook.call(self.token().runtime())?;

        self.reload(&cid)?;
        let ret = self.token().transfer_from_return(hook_ret)?;

        Ok(ret)
    }

    fn increase_allowance(
        &mut self,
        params: IncreaseAllowanceParams,
    ) -> Result<AllowanceReturn, RuntimeError> {
        let owner = self.caller_address();
        let new_allowance =
            self.token().increase_allowance(&owner, &params.operator, &params.increase)?;
        Ok(new_allowance)
    }

    fn decrease_allowance(
        &mut self,
        params: DecreaseAllowanceParams,
    ) -> Result<AllowanceReturn, RuntimeError> {
        let owner = self.caller_address();
        let new_allowance =
            self.token().decrease_allowance(&owner, &params.operator, &params.decrease)?;
        Ok(new_allowance)
    }

    fn revoke_allowance(&mut self, params: RevokeAllowanceParams) -> Result<(), RuntimeError> {
        let owner = self.caller_address();
        self.token().revoke_allowance(&owner, &params.operator)?;
        Ok(())
    }

    fn allowance(&mut self, params: GetAllowanceParams) -> Result<AllowanceReturn, RuntimeError> {
        let allowance = self.token().allowance(&params.owner, &params.operator)?;
        Ok(allowance)
    }

    fn burn(&mut self, params: BurnParams) -> Result<BurnReturn, RuntimeError> {
        let caller = self.caller_address();
        let res = self.token().burn(&caller, &params.amount)?;
        Ok(res)
    }

    fn burn_from(
        &mut self,
        params: frc46_token::token::types::BurnFromParams,
    ) -> Result<BurnFromReturn, RuntimeError> {
        let caller = self.caller_address();
        let res = self.token().burn_from(&caller, &params.owner, &params.amount)?;
        Ok(res)
    }
}

#[derive(Serialize_tuple, Deserialize_tuple, Clone, Debug)]
pub struct MintParams {
    pub initial_owner: Address,
    pub amount: TokenAmount,
    pub operator_data: RawBytes,
}

impl<S: Syscalls + Clone, BS: Blockstore + Clone> FactoryToken<S, BS> {
    pub fn new(
        runtime: ActorRuntime<S, BS>,
        name: String,
        symbol: String,
        granularity: u64,
        minter: Option<ActorID>,
    ) -> Self {
        FactoryToken {
            state: FactoryTokenState {
                token: TokenState::new(&runtime).unwrap(),
                name,
                symbol,
                granularity,
                minter,
            },
            runtime,
        }
    }

    pub fn caller_address(&self) -> Address {
        let caller = self.runtime.caller();
        Address::new_id(caller)
    }

    pub fn token(&mut self) -> Token<'_, S, BS> {
        Token::wrap(self.runtime.clone(), self.state.granularity, &mut self.state.token)
    }

    pub fn load(runtime: ActorRuntime<S, BS>, cid: &Cid) -> Result<Self, RuntimeError> {
        Ok(FactoryToken { state: FactoryTokenState::load(&runtime, cid)?, runtime })
    }

    pub fn save(&self) -> Result<Cid, RuntimeError> {
        let serialized = fvm_ipld_encoding::to_vec(&self.state)
            .map_err(|err| RuntimeError::Serialization(err.to_string()))?;
        let block = Block { codec: DAG_CBOR, data: serialized };
        self.runtime
            .put(Code::Blake2b256, &block)
            .map_err(|err| RuntimeError::Serialization(err.to_string()))
    }

    fn reload(&mut self, initial_cid: &Cid) -> Result<(), RuntimeError> {
        let new_cid = self.runtime.root_cid().map_err(|_| RuntimeError::NoState(NoStateError))?;
        if new_cid != *initial_cid {
            let new_state = FactoryTokenState::load(&self.runtime, &new_cid)?;
            let _old = std::mem::replace(&mut self.state, new_state);
        }
        Ok(())
    }

    pub fn mint(&mut self, params: MintParams) -> Result<MintReturn, RuntimeError> {
        // check if the caller matches our authorise mint operator
        // no minter address means minting has been permanently disabled
        let minter = self.state.minter.ok_or(RuntimeError::MintingDisabled)?;
        let caller_id = self.runtime.caller(); // TODO: may need to add this to ActorRuntime
        if caller_id != minter {
            return Err(RuntimeError::AddressNotAuthorized);
        }

        let mut hook = self.token().mint(
            &Address::new_id(caller_id),
            &params.initial_owner,
            &params.amount,
            params.operator_data,
            Default::default(),
        )?;

        let cid = self.save()?;
        self.runtime.set_root(&cid).map_err(|_| NoStateError)?;

        let hook_ret = hook.call(self.token().runtime())?;

        self.reload(&cid)?;
        let ret = self.token().mint_return(hook_ret)?;

        Ok(ret)
    }

    /// Permanently disable minting
    /// Only the authorised mint operator can do this
    pub fn disable_mint(&mut self) -> Result<(), RuntimeError> {
        // no minter means minting has already been permanently disabled
        // we return this if already disabled because it will make more sense than failing the address check below
        let minter = self.state.minter.ok_or(RuntimeError::MintingDisabled)?;
        let caller_id = self.runtime.caller(); // TODO: add this to ActorRuntime
        if caller_id != minter {
            return Err(RuntimeError::AddressNotAuthorized);
        }

        self.state.minter = None;
        Ok(())
    }
}

pub fn deserialize_params<O: DeserializeOwned>(params: u32) -> O {
    let params = sdk::message::params_raw(params).unwrap();
    let params = params.unwrap();
    params.deserialize().unwrap()
}

pub fn return_ipld<T>(value: &T) -> std::result::Result<u32, RuntimeError>
where
    T: Serialize + ?Sized,
{
    let bytes = fvm_ipld_encoding::to_vec(value)?;
    Ok(sdk::ipld::put_block(DAG_CBOR, bytes.as_slice())?)
}

/// Generic invoke for FRC46 Token methods
/// Given a method number and parameter block id, invokes the appropriate method on the FRC46Token interface
///
/// The flush_state function passed into this must flush current state to the blockstore and update the root cid
/// This is called after operations which mutate the state, such as changing an allowance or burning tokens.
///
/// Transfer and TransferFrom operations invoke the receiver hook which will require flushing state before calling the hook
/// This must be done inside the FRC46Token::transfer/transfer_from functions
///
/// Possible returns:
/// - Ok(None) - method not found
/// - Ok(Some(u32)) - block id of results saved to blockstore (or NO_DATA_BLOCK_ID if there is no result to return)
/// - Err(error) - any error encountered during operation
///
pub fn frc46_invoke<T, F, E>(
    method_num: u64,
    params: u32,
    token: &mut T,
    flush_state: F,
) -> Result<Option<u32>, E>
where
    T: FRC46Token<TokenError = E>,
    F: FnOnce(&mut T) -> Result<(), E>,
{
    match_method!(method_num, {
        "Name" => {
            Ok(frc46_return_block(&token.name()))
        }
        "Symbol" => {
            Ok(frc46_return_block(&token.symbol()))
        }
        "TotalSupply" => {
            Ok(frc46_return_block(&token.total_supply()))
        }
        "BalanceOf" => {
            let params = frc46_unpack_params(params);
            let res = token.balance_of(params)?;
            Ok(frc46_return_block(&res))
        }
        "Allowance" => {
            let params = frc46_unpack_params(params);
            let res = token.allowance(params)?;
            Ok(frc46_return_block(&res))
        }
        "IncreaseAllowance" => {
            let params = frc46_unpack_params(params);
            let res = token.increase_allowance(params)?;
            flush_state(token)?;
            Ok(frc46_return_block(&res))
        }
        "DecreaseAllowance" => {
            let params = frc46_unpack_params(params);
            let res = token.decrease_allowance(params)?;
            flush_state(token)?;
            Ok(frc46_return_block(&res))
        }
        "RevokeAllowance" => {
            let params = frc46_unpack_params(params);
            token.revoke_allowance(params)?;
            flush_state(token)?;
            Ok(Some(NO_DATA_BLOCK_ID))
        }
        "Burn" => {
            let params = frc46_unpack_params(params);
            let res = token.burn(params)?;
            flush_state(token)?;
            Ok(frc46_return_block(&res))

        }
        "TransferFrom" => {
            let params = frc46_unpack_params(params);
            let res = token.transfer_from(params)?;
            Ok(frc46_return_block(&res))
        }
        "Transfer" => {
            let params = frc46_unpack_params(params);
            let res = token.transfer(params)?;
            Ok(frc46_return_block(&res))
        }
        _ => {
            // no method found - it's not considered an error here, but an upstream caller may choose to treat it as one
            Ok(None)
        }
    })
}

// deserialise params for passing to token methods
// this aborts on errors and is intended for frc46_invoke to use
pub fn frc46_unpack_params<O: DeserializeOwned>(params: u32) -> O {
    let params = match sdk::message::params_raw(params) {
        Ok(Some(params)) => params,
        Ok(None) => {
            fvm_sdk::vm::abort(
                ExitCode::USR_ILLEGAL_ARGUMENT.value(),
                Some(String::from("missing parameters").as_str()),
            );
        }
        Err(e) => {
            fvm_sdk::vm::abort(
                ExitCode::USR_SERIALIZATION.value(),
                Some(format!("failed to get raw params {e}").as_str()),
            );
        }
    };

    match params.deserialize() {
        Ok(p) => p,
        Err(e) => {
            fvm_sdk::vm::abort(
                ExitCode::USR_SERIALIZATION.value(),
                Some(format!("failed to deserialize params {e}").as_str()),
            );
        }
    }
}

// serialise and save return data to the blockstore
// this also aborts on error and is intended for frc46_invoke to use
pub fn frc46_return_block<T>(value: &T) -> Option<u32>
where
    T: Serialize + ?Sized,
{
    let bytes = match fvm_ipld_encoding::to_vec(value) {
        Ok(b) => b,
        Err(e) => {
            fvm_sdk::vm::abort(
                ExitCode::USR_SERIALIZATION.value(),
                Some(format!("failed to serialise return data {e}").as_str()),
            );
        }
    };

    Some(sdk::ipld::put_block(DAG_CBOR, bytes.as_slice()).unwrap_or_else(|e| {
        fvm_sdk::vm::abort(
            ExitCode::USR_SERIALIZATION.value(),
            Some(format!("failed to serialise return data {e}").as_str()),
        )
    }))
}

#[cfg(test)]
mod test {
    use frc46_token::token::types::FRC46Token;
    use fvm_actor_utils::{
        shared_blockstore::SharedMemoryBlockstore, syscalls::fake_syscalls::FakeSyscalls,
        util::ActorRuntime,
    };
    use fvm_ipld_encoding::RawBytes;
    use fvm_shared::{address::Address, econ::TokenAmount};

    use crate::{FactoryToken, MintParams, RuntimeError};

    const ALICE: Address = Address::new_id(1);
    const BOB: Address = Address::new_id(2);

    // set up a token instance for testing
    // fake syscalls will always return actor id 1 as the caller
    // so we'd typically want that as the minter (unless we want to be denied)
    fn setup_token(minter: &Address) -> FactoryToken<FakeSyscalls, SharedMemoryBlockstore> {
        let runtime =
            ActorRuntime::<FakeSyscalls, SharedMemoryBlockstore>::new_shared_test_runtime();
        let actor_id = runtime.resolve_id(&minter).unwrap();
        FactoryToken::new(
            runtime,
            String::from("Test Token"),
            String::from("TEST"),
            1,
            Some(actor_id),
        )
    }

    #[test]
    fn it_mints() {
        let mut token = setup_token(&ALICE);

        let ret = token
            .mint(MintParams {
                initial_owner: BOB,
                amount: TokenAmount::from_whole(10),
                operator_data: RawBytes::default(),
            })
            .unwrap();

        // check balance
        assert_eq!(ret.balance, TokenAmount::from_whole(10));
        assert_eq!(token.balance_of(BOB).unwrap(), TokenAmount::from_whole(10));
    }

    #[test]
    fn it_denies_unauthorised_minter() {
        let mut token = setup_token(&BOB);

        let err = token
            .mint(MintParams {
                initial_owner: ALICE,
                amount: TokenAmount::from_whole(10),
                operator_data: RawBytes::default(),
            })
            .unwrap_err();

        // check error
        match err {
            RuntimeError::AddressNotAuthorized => {}
            _ => panic!("unexpected error"),
        }
    }
}
