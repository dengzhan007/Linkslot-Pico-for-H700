#include <arch/arm/mmu.h>
#include <arch/ops.h>
#include <stdint.h>

/*
 * GammaOS LK calls platform_init_mmu_mappings() from arch_early_init(),
 * before target_early_init(). The hook must own the complete AArch32 mapping
 * setup, not merely edit one descriptor:
 *
 *   1. initialize the LK section translation table;
 *   2. map the verified RG34XX 1 GiB DRAM range as normal write-back memory;
 *   3. map the H700 SoC MMIO 1 MiB window (SYS_CFG, PIO, CCU) as shared Device memory;
 *   4. enable the MMU only after all identity mappings are installed.
 *
 * DRAM base/size are the live RG34XX inventory values. These mappings do
 * not allocate or reserve memory and do not authorize any BL33 heap/stack,
 * page-table, framebuffer, payload or DMA placement within the DRAM range.
 *
 * This board currently builds the non-LM, non-3-level AArch32 LK path.
 */
#if defined(MTK_3LEVEL_PAGETABLE) || defined(MTK_LM_MODE)
#error "RG34XX H700 MMU mapping requires the reviewed AArch32 section-table path"
#endif

#define H700_DRAM_BASE 0x40000000u
#define H700_DRAM_SIZE 0x40000000u
#define H700_MMU_SECTION_SIZE 0x00100000u

#define H700_SOC_MMIO_WINDOW_BASE 0x03000000u
#define H700_SYS_CFG_VER_REG 0x03000024u
#define H700_RG34XX_PIO_BASE 0x0300b000u

_Static_assert((H700_SYS_CFG_VER_REG & ~(H700_MMU_SECTION_SIZE - 1u)) ==
                   H700_SOC_MMIO_WINDOW_BASE,
               "SYS_CFG VER_REG must remain inside the mapped SoC MMIO section");
_Static_assert((H700_RG34XX_PIO_BASE & ~(H700_MMU_SECTION_SIZE - 1u)) ==
                   H700_SOC_MMIO_WINDOW_BASE,
               "RG34XX PIO must remain inside the mapped SoC MMIO section");

void platform_init_mmu_mappings(void)
{
    uint32_t address;

    /* arm_mmu_init() builds LK's 4096-entry identity section table and TTBR. */
    arm_mmu_init();

    /* The RG34XX board has 1 GiB LPDDR4 at [0x40000000, 0x80000000). */
    for (address = H700_DRAM_BASE;
         address < H700_DRAM_BASE + H700_DRAM_SIZE;
         address += H700_MMU_SECTION_SIZE) {
        arm_mmu_map_section(
            address,
            address,
            MMU_MEMORY_TYPE_NORMAL_WRITE_BACK_ALLOCATE |
                MMU_MEMORY_AP_READ_WRITE);
    }

    /* This Device section covers SYS_CFG/VER_REG, PIO and CCU. */
    arm_mmu_map_section(
        H700_SOC_MMIO_WINDOW_BASE,
        H700_SOC_MMIO_WINDOW_BASE,
        MMU_MEMORY_TYPE_DEVICE_SHARED |
            MMU_MEMORY_AP_READ_WRITE |
            MMU_MEMORY_XN);

    /* Match GammaOS LK's platform contract: do not enable MMU before maps. */
    arch_enable_mmu();
}
