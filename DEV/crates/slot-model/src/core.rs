//! 核心表：9 个 libretro 核心的元数据。
//!
//! 只放**静态事实**：id、动态库名、显示名、芯片短名、可运行的机种、固定选项、
//! 颜色校正键名、是否支持连机。依赖运行时上下文的部分（调色板、BIOS、序列号、
//! 连机玩家号）仍由 `slot/src/core.rs` 按上下文补齐。

use crate::machine::MachineId;

/// 核心 id：与 `System/<id>_libretro.so`、`States/<platform>/<id>/` 一致。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CoreId(pub &'static str);

impl CoreId {
    pub fn as_str(self) -> &'static str {
        self.0
    }
}

/// libretro 的颜色校正选项（键名与开/关取值每个核心都不一样）。
#[derive(Clone, Copy, Debug)]
pub struct ColourSpec {
    pub key: &'static str,
    pub on: &'static str,
    pub off: &'static str,
}

/// 一个核心的全部静态元数据。
#[derive(Debug)]
pub struct CoreSpec {
    pub id: CoreId,
    /// 动态库文件名前缀（`System/<dylib>_libretro.so`）。
    pub dylib: &'static str,
    /// 显示名（设置页 / PCB 大字号）。
    pub display: &'static str,
    /// PCB 插槽里的短名。
    pub chip: &'static str,
    /// 可运行的机种。
    pub machines: &'static [MachineId],
    /// 是否接受 `<dylib>_frameskip = auto`（未声明该选项的核心会告警）。
    pub frameskip: bool,
    /// 是否支持 mGBA 的连机选项（`mgba_link` 电缆）。
    pub link: bool,
    /// 是否通过 libretro 的 netpacket 接口承载联机。
    ///
    /// gpSP 的每游戏 netpacket 连接、以及 armGBA 的 Link Mode Separate
    /// （电缆或无线适配器）都走这个接口；mGBA 的电缆走上面的 `link` 那条，
    /// 不是 netpacket。
    pub netpacket: bool,
    /// 与上下文无关的固定选项（libretro 键名 + 值）。
    pub fixed_options: &'static [(&'static str, &'static str)],
    pub colour: Option<ColourSpec>,
}

pub const MGBA: CoreSpec = CoreSpec {
    id: CoreId("mgba"),
    dylib: "mgba",
    display: "mGBA",
    chip: "mGBA",
    machines: &[MachineId("GBA"), MachineId("GB"), MachineId("GBC")],
    frameskip: true,
    link: true,
    netpacket: false,
    fixed_options: &[("mgba_sgb_borders", "OFF"), ("mgba_force_gbp", "ON")],
    colour: Some(ColourSpec {
        key: "mgba_color_correction",
        on: "Auto",
        off: "OFF",
    }),
};

pub const GPSP: CoreSpec = CoreSpec {
    id: CoreId("gpsp"),
    dylib: "gpsp",
    display: "gpSP",
    chip: "gpSP",
    machines: &[MachineId("GBA")],
    frameskip: true,
    link: false,
    netpacket: true,
    // 序列号与 BIOS 启动模式依赖运行时上下文，见 slot/src/core.rs。
    fixed_options: &[("gpsp_rtc_time_source", "system")],
    colour: Some(ColourSpec {
        key: "gpsp_color_correction",
        on: "enabled",
        off: "disabled",
    }),
};

pub const FCEUMM: CoreSpec = CoreSpec {
    id: CoreId("fceumm"),
    dylib: "fceumm",
    display: "fceumm",
    chip: "fceumm",
    machines: &[MachineId("FC")],
    frameskip: false,
    link: false,
    netpacket: false,
    // overscan 的值是像素数：fceumm 用 atoi，写 "enabled" 会被当成 0。
    fixed_options: &[
        ("fceumm_overscan_v_top", "8"),
        ("fceumm_overscan_v_bottom", "8"),
        ("fceumm_turbo_enable", "Player 1"),
    ],
    colour: None,
};

/// Super Famicom / SNES 的官方随包核心：GPL-2.0，构建时自编译（glibc ≤ 2.31）。
/// Famicom/NES 的高兼容可选核心：不随包发布，用 LSP 扩展包安装。
pub const NESTOPIA: CoreSpec = CoreSpec {
    id: CoreId("nestopia"),
    dylib: "nestopia",
    display: "Nestopia",
    chip: "Nestopia",
    machines: &[MachineId("FC")],
    frameskip: false,
    link: false,
    netpacket: false,
    fixed_options: &[],
    colour: None,
};

pub const SNES9X: CoreSpec = CoreSpec {
    id: CoreId("snes9x"),
    dylib: "snes9x",
    display: "Snes9x",
    chip: "snes9x",
    machines: &[MachineId("SFC")],
    frameskip: false,
    link: false,
    netpacket: false,
    fixed_options: &[],
    colour: None,
};

pub const SUPAFAUST: CoreSpec = CoreSpec {
    id: CoreId("supafaust"),
    dylib: "supafaust",
    display: "supafaust",
    chip: "supafaust",
    machines: &[MachineId("SFC")],
    frameskip: false,
    link: false,
    netpacket: false,
    fixed_options: &[],
    colour: None,
};

pub const GENESIS: CoreSpec = CoreSpec {
    id: CoreId("genesis_plus_gx"),
    dylib: "genesis_plus_gx",
    display: "Genesis Plus GX",
    chip: "GPGX",
    machines: &[MachineId("MD")],
    frameskip: false,
    link: false,
    netpacket: false,
    fixed_options: &[
        ("genesis_plus_gx_overscan", "disabled"),
        ("genesis_plus_gx_render", "single field"),
    ],
    colour: None,
};

/// GBA 的第三个核心：armGBA（AArch64 重编译器）。以 LSP 扩展包安装。
///
/// 上游 libretro 核心实现了 `SET_NETPACKET_INTERFACE`（协议 `armGBA link 3`）：
/// `Link Mode = separate` 时以 netpacket 承载 Link Cable / Wireless Adapter，
/// 可以接 linkSlot 的 2 人联机会话。Together（一台设备跑所有主机）不接。
pub const ARMGBA: CoreSpec = CoreSpec {
    id: CoreId("armgba"),
    dylib: "armgba",
    display: "armGBA",
    chip: "armGBA",
    machines: &[MachineId("GBA")],
    frameskip: false,
    link: false,
    netpacket: true,
    fixed_options: &[],
    colour: Some(ColourSpec {
        key: "armgba_color_correction",
        on: "enabled",
        off: "disabled",
    }),
};

/// FBNeo：街机与 NeoGeo 的唯一核心，由平台扩展包提供（不随 app 发布）。
///
/// 它跑的是 ZIP 里的 ROM 集而不是单个卡带文件，因此既不接受 frameskip 选项、
/// 也没有 linkslot 的联机会话；颜色校正交给核心默认值。
pub const FBNEO: CoreSpec = CoreSpec {
    id: CoreId("fbneo"),
    dylib: "fbneo",
    display: "FBNeo",
    chip: "FBNeo",
    machines: &[MachineId("ARCADE"), MachineId("NEOGEO")],
    frameskip: false,
    link: false,
    netpacket: false,
    fixed_options: &[],
    colour: None,
};

/// 核心表。顺序用于稳定输出（不是落盘契约）。
pub const CORES: &[CoreSpec] = &[
    MGBA, GPSP, FCEUMM, NESTOPIA, SNES9X, SUPAFAUST, GENESIS, ARMGBA, FBNEO,
];

pub fn core(id: &str) -> Option<&'static CoreSpec> {
    CORES.iter().find(|c| c.id.0 == id)
}

/// 能运行某机种的核心（表顺序）。
pub fn cores_for(machine: MachineId) -> impl Iterator<Item = &'static CoreSpec> {
    CORES
        .iter()
        .filter(move |c| c.machines.contains(&machine))
}

/// 所有核心 id（表顺序）。
pub fn core_ids() -> impl Iterator<Item = &'static str> {
    CORES.iter().map(|c| c.id.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netpacket_is_declared_only_by_the_cores_that_carry_a_link_over_it() {
        assert!(GPSP.netpacket);
        assert!(ARMGBA.netpacket);
        assert!(
            !MGBA.netpacket,
            "mGBA's cable is driven by mgba_link, not netpacket"
        );
        for spec in [FCEUMM, NESTOPIA, SNES9X, SUPAFAUST, GENESIS] {
            assert!(!spec.netpacket, "{} claimed netpacket", spec.id.0);
        }
    }
}
