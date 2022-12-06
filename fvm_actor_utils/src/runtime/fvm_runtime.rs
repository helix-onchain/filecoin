use fvm_ipld_encoding::RawBytes;
use fvm_sdk;
use fvm_shared::{address::Address, MethodNum};

use super::Runtime;

/// Runtime that delegates to fvm_sdk allowing actors to be deployed on-chain
#[derive(Default, Debug, Clone)]
pub struct FvmRuntime {}

impl Runtime for FvmRuntime {
    fn root(&self) -> Result<cid::Cid, fvm_sdk::error::NoStateError> {
        fvm_sdk::sself::root()
    }

    fn receiver(&self) -> fvm_shared::ActorID {
        fvm_sdk::message::receiver()
    }

    fn send(
        &self,
        to: &Address,
        method: MethodNum,
        params: RawBytes,
        value: fvm_shared::econ::TokenAmount,
    ) -> fvm_sdk::SyscallResult<fvm_shared::receipt::Receipt> {
        fvm_sdk::send::send(to, method, params, value)
    }

    fn resolve_address(&self, addr: &Address) -> Option<fvm_shared::ActorID> {
        fvm_sdk::actor::resolve_address(addr)
    }
}
