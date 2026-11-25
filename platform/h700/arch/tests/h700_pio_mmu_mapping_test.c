#include <arch/arm/mmu.h>
#include <arch/ops.h>

#include <assert.h>
#include <stdint.h>
#include <stdio.h>

#define EXPECTED_DRAM_BASE 0x40000000u
#define EXPECTED_DRAM_END  0x80000000u
#define SECTION_SIZE       0x00100000u
#define PIO_BASE           0x03000000u

static unsigned int g_call;
static unsigned int g_dram_maps;
static unsigned int g_pio_maps;
static unsigned int g_enabled;
static unsigned int g_first_dram;
static unsigned int g_last_dram;

void platform_init_mmu_mappings(void);

void arm_mmu_init(void)
{
    assert(g_call == 0u);
    g_call = 1u;
}

void arm_mmu_map_section(addr_t paddr, addr_t vaddr, unsigned int flags)
{
    assert(g_call == 1u);
    assert(paddr == vaddr);

    if (paddr >= EXPECTED_DRAM_BASE && paddr < EXPECTED_DRAM_END) {
        assert(flags == (MMU_MEMORY_TYPE_NORMAL_WRITE_BACK_ALLOCATE |
                         MMU_MEMORY_AP_READ_WRITE));
        if (g_dram_maps == 0u)
            g_first_dram = paddr;
        g_last_dram = paddr;
        g_dram_maps++;
        return;
    }

    assert(paddr == PIO_BASE);
    assert(flags == (MMU_MEMORY_TYPE_DEVICE_SHARED |
                     MMU_MEMORY_AP_READ_WRITE | MMU_MEMORY_XN));
    g_pio_maps++;
}

void arch_enable_mmu(void)
{
    assert(g_call == 1u);
    assert(g_dram_maps == 1024u);
    assert(g_pio_maps == 1u);
    assert(g_first_dram == EXPECTED_DRAM_BASE);
    assert(g_last_dram == EXPECTED_DRAM_END - SECTION_SIZE);
    g_call = 2u;
    g_enabled++;
}

int main(void)
{
    platform_init_mmu_mappings();
    assert(g_call == 2u);
    assert(g_enabled == 1u);
    assert(g_dram_maps == 1024u);
    assert(g_pio_maps == 1u);
    puts("RG34XX MMU init -> 1GiB DRAM WB map -> PIO Device map -> MMU enable: PASS");
    return 0;
}
