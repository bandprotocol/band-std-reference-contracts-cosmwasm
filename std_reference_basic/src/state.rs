use cw_storage_plus::{Item, Map};

use crate::struct_types::{Config, RefData, Relayer};

pub const CONFIG: Item<Config> = Item::new("config");
pub const RELAYERS: Map<&str, Relayer> = Map::new("relayers");
pub const REFDATA: Map<&str, RefData> = Map::new("refdata");
