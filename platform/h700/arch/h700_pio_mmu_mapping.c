#include <arch/arm/mmu.h>
#include <stdint.h>

/*
 * GammaOS LK calls platform_init_mmu_mappings() from arch_early_init().
 * Map the H700 PIO/CCU 1 MiB register window as shared device memory before
 * target_early_init() accesses PIO@0x0300b000. This is a VA=PA mapping and
 * consumes no DRAM; only the existing ARM section-table entry is updated.
 *
 * The current H700 target is the AArch32, non-LM, non-3-level LK build.
 * If the architecture mapping model changes, fail the build instead of
 * silently leaving the early LED MMIO unmapped.
 */
#if defined(MTK_3LEVEL_PAGETABLE) || defined(MTK_LM_MODE)
#error "RG34XX H700 PIO mapping requires the reviewed AArch32 section-table path"
#endif

#define H700_PIO_WINDOW_BASE 0x03000000u
#define H700_PIO_WINDOW_SIZE 0x00100000u

void platform_init_mmu_mappings(void)
{
    (void)H700_PIO_WINDOW_SIZE;
    arm_mmu_map_section(
        H700_PIO_WINDOW_BASE,
        H700_PIO_WINDOW_BASE,
        MMU_MEMORY_TYPE_DEVICE_SHARED |
            MMU_MEMORY_AP_READ_WRITE |
            MMU_MEMORY_XN);
}
