use std::{collections::HashMap, sync::Arc};

use crate::executor::interface::Executor;

pub mod inmemory;
pub mod interface;
pub mod subprocess;

pub type ExecutorRegistry = Arc<HashMap<String, Box<dyn Executor>>>;
