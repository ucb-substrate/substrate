use std::collections::HashMap;
use std::sync::Arc;

use super::query::{Query, QueryResult};
use super::spec::{MosId, MosKind, MosSpec};
use crate::error::{ErrorSource, Result};
use crate::pdk::Pdk;

pub struct MosDb {
    devices: HashMap<MosId, MosSpec>,
}

impl MosDb {
    pub(crate) fn new(pdk: Arc<dyn Pdk>) -> Result<Self> {
        let devices = pdk.mos_devices();
        let devices = HashMap::from_iter(devices.into_iter().map(|m| (m.id, m)));
        Ok(Self { devices })
    }

    pub fn query(&self, query: Query) -> Result<QueryResult> {
        self.devices
            .values()
            .find(|&m| m.supply == query.supply && m.kind == query.kind && m.flavor == query.flavor)
            .map(|m| QueryResult {
                id: m.id,
                spec: m,
                alternate: false,
            })
            .ok_or(ErrorSource::DeviceNotFound.into())
    }

    pub fn get_spec(&self, id: MosId) -> Result<&MosSpec> {
        self.devices
            .get(&id)
            .ok_or(ErrorSource::DeviceNotFound.into())
    }

    pub fn get_spec_from_name(&self, name: &str) -> Result<&MosSpec> {
        self.devices
            .values()
            .find(|&m| m.name == name)
            .ok_or(ErrorSource::DeviceNotFound.into())
    }

    pub fn default_nmos(&self) -> Result<QueryResult> {
        self.query(Query::builder().kind(MosKind::Nmos).build().unwrap())
    }

    pub fn default_pmos(&self) -> Result<QueryResult> {
        self.query(Query::builder().kind(MosKind::Pmos).build().unwrap())
    }
}
