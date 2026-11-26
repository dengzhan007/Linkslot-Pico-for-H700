#ifndef H700_PHYSICAL_MEMORY_H
#define H700_PHYSICAL_MEMORY_H

#include <stdint.h>

/*
 * Live RG34XX non-SP board geometry: 1 GiB LPDDR4 at physical base
 * 0x40000000, end-exclusive 0x80000000. This reports installed DRAM capacity
 * to LK's generic arm_mmu_init(); it does not allocate, reserve or approve
 * any part of that interval for LK, heap, stack, DMA or an application.
 */
#define H700_RG34XX_DRAM_BYTES 0x40000000ULL

uint64_t physical_memory_size(void);

#endif
