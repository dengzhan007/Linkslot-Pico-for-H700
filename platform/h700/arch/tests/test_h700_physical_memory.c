#include <stdint.h>

#include "../h700_physical_memory.h"

_Static_assert(H700_RG34XX_DRAM_BYTES == UINT64_C(0x40000000),
               "RG34XX live DRAM capacity must remain 1 GiB");

int main(void)
{
    if (physical_memory_size() != UINT64_C(0x40000000))
        return 1;
    if (physical_memory_size() + UINT64_C(0x40000000) != UINT64_C(0x80000000))
        return 2;
    return 0;
}
