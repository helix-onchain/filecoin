use fvm_ipld_encoding::ipld_block::IpldBlock;
#[cfg(not(feature = "no_sdk"))]
use fvm_sdk::send;
use fvm_shared::sys::SendFlags;
use fvm_shared::Response;
use fvm_shared::{address::Address, econ::TokenAmount, error::ErrorNumber};
use thiserror::Error;

use crate::hash::{Hasher, MethodNameErr, MethodResolver};

/// Utility to invoke standard methods on deployed actors
#[derive(Default)]
pub struct MethodMessenger<T: Hasher> {
    method_resolver: MethodResolver<T>,
}

#[derive(Error, PartialEq, Eq, Debug)]
pub enum MethodMessengerError {
    #[error("error when calculating method name: `{0}`")]
    MethodName(#[from] MethodNameErr),
    #[error("error sending message: `{0}`")]
    Syscall(#[from] ErrorNumber),
}

impl<T: Hasher> MethodMessenger<T> {
    /// Creates a new method messenger using a specified hashing function (blake2b by default)
    pub fn new(hasher: T) -> Self {
        Self { method_resolver: MethodResolver::new(hasher) }
    }

    /// Calls a method (by name) on a specified actor by constructing and publishing the underlying
    /// on-chain Message
    #[cfg(not(feature = "no_sdk"))]
    pub fn call_method(
        &self,
        to: &Address,
        method: &str,
        params: Option<IpldBlock>,
        value: TokenAmount,
    ) -> Result<Response, MethodMessengerError> {
        let method = self.method_resolver.method_number(method)?;
        send::send(to, method, params, value, None, SendFlags::default())
            .map_err(MethodMessengerError::from)
    }

    #[cfg(feature = "no_sdk")]
    #[allow(unused_variables)]
    pub fn call_method(
        &self,
        to: &Address,
        method: &str,
        params: Option<IpldBlock>,
        value: TokenAmount,
    ) -> Result<Response, MethodMessengerError> {
        let _method = self.method_resolver.method_number(method)?;
        unimplemented!()
    }
}
