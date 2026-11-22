OUTPUT_FORMAT("elf64-littleaarch64")
OUTPUT_ARCH(aarch64)
ENTRY(_start)

IMAGE_BASE = 0x40080000;
IMAGE_HEADER_BYTES = 0x40;
IMAGE_LIMIT = IMAGE_BASE + 0x1138000;

SECTIONS
{
    . = IMAGE_BASE + IMAGE_HEADER_BYTES;

    .text : ALIGN(16)
    {
        KEEP(*(.text.entry))
        *(.text .text.*)
    }

    .rodata : ALIGN(16)
    {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(16)
    {
        *(.data .data.*)
    }

    .payload_storage : ALIGN(16)
    {
        KEEP(*(.payload_storage.record))
        KEEP(*(.payload_storage.stack))
    }

    __image_end = .;

    /DISCARD/ :
    {
        *(.comment)
        *(.note*)
        *(.eh_frame*)
        *(.ARM.attributes)
    }

    ASSERT(_start == IMAGE_BASE + IMAGE_HEADER_BYTES,
           "AArch64 Image code0 must branch to _start at offset 0x40")
    ASSERT(__entry_record >= IMAGE_BASE + IMAGE_HEADER_BYTES,
           "entry record must be inside the Image")
    ASSERT(__entry_stack_top <= IMAGE_LIMIT,
           "private stack exceeds the proven stock Image size ceiling")
    ASSERT(__image_end <= IMAGE_LIMIT,
           "entry probe exceeds the conservative stock Image size ceiling")
}
