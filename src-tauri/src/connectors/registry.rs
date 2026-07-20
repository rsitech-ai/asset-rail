use std::{collections::BTreeMap, sync::Arc};

use super::{ConnectorId, ExchangeConnector};

#[derive(Default)]
pub struct ConnectorRegistry {
    connectors: BTreeMap<ConnectorId, Arc<dyn ExchangeConnector>>,
}

impl ConnectorRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a connector under its stable identifier.
    ///
    /// # Errors
    /// Returns [`RegistryError::Duplicate`] when the identifier is already registered.
    pub fn register(&mut self, connector: Arc<dyn ExchangeConnector>) -> Result<(), RegistryError> {
        let id = connector.id().clone();
        if self.connectors.contains_key(&id) {
            return Err(RegistryError::Duplicate(id));
        }
        self.connectors.insert(id, connector);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: &ConnectorId) -> Option<Arc<dyn ExchangeConnector>> {
        self.connectors.get(id).cloned()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.connectors.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.connectors.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RegistryError {
    #[error("connector identifier is already registered: {0}")]
    Duplicate(ConnectorId),
}
