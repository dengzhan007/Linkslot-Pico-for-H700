#ifndef H700_TEST_ARCH_ARM_MMU_H
#define H700_TEST_ARCH_ARM_MMU_H
typedef unsigned int addr_t;
#define MMU_MEMORY_TYPE_DEVICE_SHARED ((0x0u << 12) | (0x1u << 2))
#define MMU_MEMORY_AP_READ_WRITE (0x3u << 10)
#define MMU_MEMORY_XN (0x1u << 4)
void arm_mmu_map_section(addr_t paddr, addr_t vaddr, unsigned int flags);
#endif
