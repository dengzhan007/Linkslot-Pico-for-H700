.section .text.entry,"ax",%progbits
.align 4
.global _start
.type _start, %function

/*
 * Passive RG34XX entry probe.
 *
 * This image is only compiled and statically inspected in the current phase.
 * It is not authorized for device execution. It assumes the stock U-Boot
 * kernel slot at 0x40080000 and the candidate A64 EL1h monitor handoff.
 *
 * No memory is accessed until the private record and stack addresses are
 * materialized. The first writes stay inside this Image's linked extent.
 * No MMIO, SMC, Linux code, allocator, or external RAM is used.
 */
_start:
    /* Preserve the firmware-provided handoff arguments and original SP. */
    mov     x19, x0
    mov     x20, x1
    mov     x21, x2
    mov     x22, x3
    mov     x23, sp

    /* Capture DAIF before masking every exception class. */
    mrs     x24, daif
    mrs     x25, CurrentEL
    msr     daifset, #0xf
    isb

    /*
     * Do not touch data memory unless the monitor left both the EL1 MMU
     * and data cache disabled. With either enabled, wait in-place: no
     * stack setup and no record writes occur under an unknown translation
     * or cache-coherency regime.
     */
    mrs     x26, sctlr_el1
    tbnz    x26, #0, .Lunsafe_halt  /* SCTLR_EL1.M */
    tbnz    x26, #2, .Lunsafe_halt  /* SCTLR_EL1.C */

    /* Establish a private 16-byte-aligned stack inside this Image. */
    adrp    x9, __entry_stack_top
    add     x9, x9, :lo12:__entry_stack_top
    and     x9, x9, #0xfffffffffffffff0
    mov     sp, x9

    /* Record is also inside the loaded Image; no BSS or runtime is required. */
    adrp    x10, __entry_record
    add     x10, x10, :lo12:__entry_record
    stp     x19, x20, [x10, #0x00]   /* entry x0, x1 */
    stp     x21, x22, [x10, #0x10]   /* entry x2, x3 */
    stp     x23, x24, [x10, #0x20]   /* original SP, initial DAIF */
    str     x25, [x10, #0x30]        /* CurrentEL */

    str     x26, [x10, #0x38]       /* SCTLR_EL1 observed before memory writes */
    mrs     x11, ttbr0_el1
    str     x11, [x10, #0x40]
    mrs     x11, ttbr1_el1
    str     x11, [x10, #0x48]
    mrs     x11, tcr_el1
    str     x11, [x10, #0x50]
    mrs     x11, mair_el1
    str     x11, [x10, #0x58]
    mrs     x11, vbar_el1
    str     x11, [x10, #0x60]
    mrs     x11, mpidr_el1
    str     x11, [x10, #0x68]

    /* Publish the record only after all fields have been written. */
    movz    x11, #0x3748
    movk    x11, #0x3030, lsl #16
    movk    x11, #0x5250, lsl #32
    movk    x11, #0x3142, lsl #48
    str     x11, [x10, #0x70]
    dsb     sy

.Lunsafe_halt:
1:
    wfe
    b       1b

.size _start, . - _start

.section .payload_storage.record,"aw",%progbits
.align 4
.global __entry_record
__entry_record:
    .zero 128

.section .payload_storage.stack,"aw",%progbits
.align 4
.global __entry_stack_bottom
__entry_stack_bottom:
    .zero 4096
.global __entry_stack_top
__entry_stack_top:
