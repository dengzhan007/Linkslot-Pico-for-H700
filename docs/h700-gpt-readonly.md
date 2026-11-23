# H700 read-only GPT discovery

## Purpose and scope

The source under platform/h700/storage/gpt/ provides a bounded primary-GPT parser for the future H700 block-device layer. It receives a 512-byte sector reader callback and has no write callback or write/erase operation. It does not initialize MMC, map MMIO, mount a filesystem, modify a card, or read the backup GPT.

The parser validates the protective MBR signature and 0xEE entry, GPT 1.0 header signature/revision/size/current-LBA/reserved field, header CRC32, primary partition-array layout and CRC32, partition usable-LBA bounds, nonzero unique GUIDs, and non-overlap. It supports bounded entry arrays of up to 128 entries with entry sizes that divide a 512-byte sector. Names are returned as ASCII; non-ASCII/control UTF-16 code units become question marks. Exact-name lookup rejects ambiguous duplicate labels.

This is metadata validation, not proof that a partition contains a usable filesystem. Callers must separately inspect a volume's filesystem boot record and mount it read-only before exposing paths to Linkslot.

## RG34XX observed partition layout

The read-only inventory captured a 30,375,936-sector disk with 512-byte logical sectors. Its GPT entries were:

| GPT slot | Start LBA | End LBA | GPT name | Observed Linux mount/use |
|---:|---:|---:|---|---|
| 1 | 73,728 | 204,799 | special | special partition |
| 2 | 204,800 | 270,335 | boot-resource | boot resources |
| 3 | 270,336 | 303,103 | env | U-Boot environment |
| 4 | 303,104 | 434,175 | boot | boot image |
| 5 | 1,482,752 | 2,531,327 | rootfs | root filesystem |
| 6 | 2,531,328 | 2,793,471 | UDISK | /data (ext4) |
| 7 | 2,793,472 | 30,375,932 | primary | /mnt/sdcard (vfat; ROM/user-data volume) |

The capture demonstrates that a partition label is not a reliable substitute for observed runtime use: p6 is named UDISK in GPT but mounted at /data, while p7 is named primary but mounted at /mnt/sdcard. Kernel PARTNAME, bootargs partitions= aliases, GPT names, and mount points are distinct naming layers and can disagree. Do not hardcode Linux mmcblk0p7 or choose a volume solely because its GPT name is UDISK or primary.

The intended same-card storage controller is sdc0 at MMIO 0x04020000 based on the live MMC mapping. The GPT parser should be called only after a separately validated read-only sdc0 block reader is available. The next volume-discovery step must identify the intended vfat/FAT volume from on-disk filesystem metadata and verify its bounds against the parsed GPT entry; it must not assume a Linux partition number or label.

## Validation

Run:

    sh scripts/test-h700-gpt-readonly.sh

The host suite builds a synthetic 512-byte-sector disk matching the captured RG34XX partition layout and tests CRC failures, protective MBR validation, capacity and I/O errors, partition bounds, overlap, duplicate unique GUIDs, and ambiguous names. When the host compiler supports them, the same tests are also run with AddressSanitizer and UndefinedBehaviorSanitizer.

These are synthetic host tests. They do not validate H700 MMC register programming, live card reads, filesystem mounting, device write safety, power-loss behavior, or boot/handoff. No firmware or storage write is part of this change.
