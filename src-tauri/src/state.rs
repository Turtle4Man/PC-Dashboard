use std::sync::RwLock;

use crate::events::VitalsPayload;

#[derive(Default)]
pub struct VitalsState {
    pub latest: RwLock<Option<VitalsPayload>>,
}

impl VitalsState {
    pub fn set(&self, payload: VitalsPayload) {
        if let Ok(mut guard) = self.latest.write() {
            *guard = Some(payload);
        }
    }

    pub fn get(&self) -> Option<VitalsPayload> {
        self.latest.read().ok().and_then(|g| g.clone())
    }
}
