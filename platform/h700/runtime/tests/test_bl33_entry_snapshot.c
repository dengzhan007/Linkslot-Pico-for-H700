#include <stddef.h>
#include <stdint.h>

#include "../h700_bl33_entry_snapshot.h"

_Static_assert(sizeof(h700_bl33_entry_snapshot_t) == 20u,
               "BL33 entry snapshot ABI must be five 32-bit words");
_Static_assert(offsetof(h700_bl33_entry_snapshot_t, entry_sp) == 0u,
               "entry SP offset changed");
_Static_assert(offsetof(h700_bl33_entry_snapshot_t, entry_lr) == 4u,
               "entry LR offset changed");
_Static_assert(offsetof(h700_bl33_entry_snapshot_t, entry_r4) == 8u,
               "entry r4 offset changed");
_Static_assert(offsetof(h700_bl33_entry_snapshot_t, entry_cpsr) == 12u,
               "entry CPSR offset changed");
_Static_assert(offsetof(h700_bl33_entry_snapshot_t, magic) == 16u,
               "snapshot magic must be published last at word five");

int main(void)
{
    h700_bl33_entry_snapshot_t result;

    if (h700_bl33_entry_snapshot_read((h700_bl33_entry_snapshot_t *)0) !=
        H700_BL33_ENTRY_SNAPSHOT_EINVAL)
        return 1;

    h700_bl33_entry_snapshot.entry_sp = 0x4a0f0000u;
    h700_bl33_entry_snapshot.entry_lr = 0x48001234u;
    h700_bl33_entry_snapshot.entry_r4 = 0x4a300000u;
    h700_bl33_entry_snapshot.entry_cpsr = 0x000001d3u;
    h700_bl33_entry_snapshot.magic = 0u;

    if (h700_bl33_entry_snapshot_read(&result) !=
        H700_BL33_ENTRY_SNAPSHOT_ENOTREADY)
        return 2;

    h700_bl33_entry_snapshot.magic = H700_BL33_ENTRY_SNAPSHOT_MAGIC;
    if (h700_bl33_entry_snapshot_read(&result) != H700_BL33_ENTRY_SNAPSHOT_OK)
        return 3;

    if (result.entry_sp != 0x4a0f0000u ||
        result.entry_lr != 0x48001234u ||
        result.entry_r4 != 0x4a300000u ||
        result.entry_cpsr != 0x000001d3u ||
        result.magic != H700_BL33_ENTRY_SNAPSHOT_MAGIC)
        return 4;

    return 0;
}
