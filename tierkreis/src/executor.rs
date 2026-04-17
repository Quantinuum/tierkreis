pub mod inmemory;
pub mod interface;
pub mod subprocess;

use std::{collections::HashMap, sync::Arc};

pub use crate::executor::inmemory::InMemoryExecutor;
pub use crate::executor::interface::Executor;
pub use crate::executor::subprocess::SubprocessExecutor;

pub type ExecutorRegistry = Arc<HashMap<String, Box<dyn Executor>>>;
