#include "h700_bl33_entry_snapshot.h"

volatile h700_bl33_entry_snapshot_t h700_bl33_entry_snapshot;

int h700_bl33_entry_snapshot_read(h700_bl33_entry_snapshot_t *out)
{
    if (out == (h700_bl33_entry_snapshot_t *)0)
        return H700_BL33_ENTRY_SNAPSHOT_EINVAL;

    /*
     * CRT0 writes the magic word last, after the other four words. This
     * service is consumed during the single-core early-boot diagnostic path;
     * it does not provide inter-core synchronization or authorize MMIO.
     */
    if (h700_bl33_entry_snapshot.magic != H700_BL33_ENTRY_SNAPSHOT_MAGIC)
        return H700_BL33_ENTRY_SNAPSHOT_ENOTREADY;

    out->entry_sp = h700_bl33_entry_snapshot.entry_sp;
    out->entry_lr = h700_bl33_entry_snapshot.entry_lr;
    out->entry_r4 = h700_bl33_entry_snapshot.entry_r4;
    out->entry_cpsr = h700_bl33_entry_snapshot.entry_cpsr;
    out->magic = h700_bl33_entry_snapshot.magic;
    return H700_BL33_ENTRY_SNAPSHOT_OK;
}
