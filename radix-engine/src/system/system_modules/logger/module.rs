use crate::system::module::SystemModule;

use crate::kernel::kernel_callback_api::KernelCallbackObject;
use radix_engine_interface::types::Level;
use sbor::rust::string::String;
use sbor::rust::vec::Vec;

#[derive(Debug, Clone)]
pub struct LoggerModule(Vec<(Level, String)>);

impl Default for LoggerModule {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl LoggerModule {
    pub fn add(&mut self, level: Level, message: String) {
        self.0.push((level, message))
    }

    pub fn finalize(self) -> Vec<(Level, String)> {
        self.0
    }
}

impl<K: KernelCallbackObject> SystemModule<K> for LoggerModule {}
