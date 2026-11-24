#![no_std]
//! linkSlot 领域契约（L1）。
//!
//! 这里只放**纯数据与派生函数**：机种表、核心表、存档单元布局、协议常量。
//! 不读文件、不碰环境、不依赖任何 crate —— 这样 `slot-store`、`slot-ui`、
//! `linkslot` 都能共用同一份事实，而不必各自维护影子表。

extern crate alloc;

#[cfg(feature = "host-compat")]
extern crate std;

pub mod core;
pub mod layout;
pub mod machine;
pub mod power;

pub use core::{core, core_ids, cores_for, ColourSpec, CoreId, CoreSpec, CORES};
#[cfg(feature = "host-compat")]
pub use layout::resolve_root;
pub use layout::DEVICE_ROOT;
pub use power::{Battery, Charge};
pub use machine::{
    by_dir, by_id, machine, Machine, MachineArt, MachineId, Platform, QuickSwitch, MACHINES,
};


#[cfg(test)]
mod h700_no_std_contract_tests {
    use alloc::vec::Vec;
    use crate::{by_dir, by_id, core, MachineId, Platform, MACHINES};

    #[test]
    fn machine_ids_and_dirs_are_unique_and_stable() {
        for (i, item) in MACHINES.iter().enumerate() {
            for other in &MACHINES[i + 1..] {
                assert_ne!(item.id, other.id);
                assert_ne!(item.dir, other.dir);
            }
        }
        for item in MACHINES {
            assert_eq!(by_id(item.id).map(|m| m.id), Some(item.id));
            assert_eq!(by_dir(item.dir).map(|m| m.id), Some(item.id));
        }
    }

    #[test]
    fn every_machine_has_registered_default_and_supported_cores() {
        for machine in MACHINES {
            assert!(machine.cores.contains(&machine.default_core));
            for id in machine.cores {
                assert!(core(id).is_some(), "unregistered core: {id}");
            }
        }
        assert_eq!(core("mgba").map(|c| c.display), Some("mGBA"));
    }

    #[test]
    fn platform_order_preserves_persisted_machine_slots() {
        assert_eq!(Platform::ALL.len(), MACHINES.len());
        for (index, platform) in Platform::ALL.iter().enumerate() {
            assert_eq!(platform.index(), index);
            assert_eq!(platform.machine().id, MACHINES[index].id);
        }
        assert_eq!(Platform::Fc.index(), 3);
        assert!(by_id(MachineId("NOT-A-MACHINE")).is_none());
    }

    #[test]
    fn rom_extensions_remain_unique_and_ordered() {
        let extensions: Vec<&str> = crate::machine::all_rom_ext().collect();
        assert_eq!(extensions.first(), Some(&"gba"));
        let mut unique = extensions.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), extensions.len());
    }

    #[test]
    fn core_registry_contains_every_1_0_45_entry_and_armgba_metadata() {
        for (index, item) in crate::CORES.iter().enumerate() {
            for other in &crate::CORES[index + 1..] {
                assert_ne!(item.id.0, other.id.0, "duplicate core id: {}", item.id.0);
            }
        }
        let armgba = crate::core("armgba").expect("armGBA metadata must be registered");
        assert_eq!(armgba.dylib, "armgba");
        assert_eq!(armgba.display, "armGBA");
        let colour = armgba.colour.expect("armGBA colour setting must be preserved");
        assert_eq!(colour.key, "armgba_color_correction");
        assert_eq!(colour.on, "enabled");
        assert_eq!(colour.off, "disabled");
    }
}
