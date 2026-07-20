use std::{fmt, marker::PhantomData};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::connectors::{ConnectorId, CredentialScheme};

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CredentialMode {
    Planning,
    Execution,
    Trading,
}

mod sealed {
    pub trait Sealed {}
}

pub trait CredentialKind: sealed::Sealed {
    const MODE: CredentialMode;
}

#[derive(Clone, Copy, Debug)]
pub struct Planning;

#[derive(Clone, Copy, Debug)]
pub struct Execution;

#[derive(Clone, Copy, Debug)]
pub struct Trading;

impl sealed::Sealed for Planning {}
impl sealed::Sealed for Execution {}
impl sealed::Sealed for Trading {}

impl CredentialKind for Planning {
    const MODE: CredentialMode = CredentialMode::Planning;
}

impl CredentialKind for Execution {
    const MODE: CredentialMode = CredentialMode::Execution;
}

impl CredentialKind for Trading {
    const MODE: CredentialMode = CredentialMode::Trading;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialScope {
    connector_id: ConnectorId,
    scheme: CredentialScheme,
}

impl CredentialScope {
    #[must_use]
    pub const fn new(connector_id: ConnectorId, scheme: CredentialScheme) -> Self {
        Self {
            connector_id,
            scheme,
        }
    }

    #[must_use]
    pub const fn connector_id(&self) -> &ConnectorId {
        &self.connector_id
    }

    #[must_use]
    pub const fn scheme(&self) -> CredentialScheme {
        self.scheme
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct CredentialHandle<Mode> {
    account_id: String,
    scope: CredentialScope,
    mode: PhantomData<fn() -> Mode>,
}

impl<Mode: CredentialKind> CredentialHandle<Mode> {
    #[must_use]
    pub const fn mode(&self) -> CredentialMode {
        Mode::MODE
    }

    #[must_use]
    pub const fn mode_for_type() -> CredentialMode {
        Mode::MODE
    }

    #[must_use]
    pub const fn scope(&self) -> &CredentialScope {
        &self.scope
    }
}

impl CredentialHandle<Planning> {
    pub(super) fn from_account_id(account_id: String, scope: CredentialScope) -> Self {
        Self {
            account_id,
            scope,
            mode: PhantomData,
        }
    }

    pub(super) fn account_id(&self) -> &str {
        &self.account_id
    }
}

impl<Mode: CredentialKind> fmt::Debug for CredentialHandle<Mode> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialHandle")
            .field("mode", &Mode::MODE)
            .field("connector_id", self.scope.connector_id())
            .field("scheme", &self.scope.scheme())
            .finish_non_exhaustive()
    }
}
