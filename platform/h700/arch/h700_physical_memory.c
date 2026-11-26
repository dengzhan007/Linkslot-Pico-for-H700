#include "h700_physical_memory.h"

/*
 * Pinned GammaOS LK arch/arm/mmu.c weakly calls physical_memory_size() to
 * decide the extent of its initial identity section map. Without this
 * provider it maps all 4096 MiB sections. The H700 target is the RG34XX
 * non-SP SKU whose live Boot0 DRAM words and live Linux FDT agree on 1 GiB.
 *
 * This is physical geometry only. H700_MEMORY_LAYOUT_APPROVED remains a
 * separate fail-closed gate for linker/load/heap/stack/page-table ownership.
 */
uint64_t physical_memory_size(void)
{
    return H700_RG34XX_DRAM_BYTES;
}
