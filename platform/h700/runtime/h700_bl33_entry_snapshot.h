#ifndef H700_BL33_ENTRY_SNAPSHOT_H
#define H700_BL33_ENTRY_SNAPSHOT_H

#include <stdint.h>

#define H700_BL33_ENTRY_SNAPSHOT_MAGIC UINT32_C(0x48374553) /* "H7ES" */

enum h700_bl33_entry_snapshot_status {
    H700_BL33_ENTRY_SNAPSHOT_OK = 0,
    H700_BL33_ENTRY_SNAPSHOT_EINVAL = -1,
    H700_BL33_ENTRY_SNAPSHOT_ENOTREADY = -2
};

/*
 * Written by the A32 CRT0 wrapper after BSS is cleared and before kmain().
 * Values are the untouched BL31->BL33 register values captured at the first
 * instruction of lk_bl33_entry. CPSR does not, by itself, identify Security
 * state or prove GIC/EL2 access permissions.
 */
typedef struct h700_bl33_entry_snapshot {
    uint32_t entry_sp;
    uint32_t entry_lr;
    uint32_t entry_r4;
    uint32_t entry_cpsr;
    uint32_t magic;
} h700_bl33_entry_snapshot_t;

/* CRT0 owns initialization; the reader never touches hardware registers. */
extern volatile h700_bl33_entry_snapshot_t h700_bl33_entry_snapshot;

int h700_bl33_entry_snapshot_read(h700_bl33_entry_snapshot_t *out);

#endif
