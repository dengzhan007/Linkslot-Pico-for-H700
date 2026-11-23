import contextlib
import io
import json
import random
import shutil
import subprocess
import struct
import tempfile
import unittest
from pathlib import Path

import h700_boot_image as boot


def fake_kernel(size=128 * 1024):
    rng = random.Random(0x353)
    data = bytearray(rng.randrange(256) for _ in range(size))
    repeated = b"H700-LINKSLOT-BOOT-PROBE-" * 256
    data[4096:4096 + len(repeated)] = repeated
    struct.pack_into("<Q", data, 8, 0x80000)
    struct.pack_into("<Q", data, 16, size)
    data[56:60] = b"ARM\x64"
    return bytes(data)


def fake_header():
    header = bytearray(boot.PAGE_SIZE)
    header[:8] = boot.ANDROID_MAGIC
    for offset, value in ((12, boot.ANDROID_KERNEL_ADDR), (20, boot.ANDROID_RAMDISK_ADDR),
                          (28, boot.ANDROID_SECOND_ADDR), (32, boot.ANDROID_TAGS_ADDR),
                          (36, boot.PAGE_SIZE), (40, 2),
                          (boot.ANDROID_HEADER_SIZE_OFFSET, boot.ANDROID_V2_HEADER_SIZE)):
        struct.pack_into("<I", header, offset, value)
    return bytes(header)


def fake_dtb():
    dtb = bytearray(40)
    dtb[:4] = b"\xd0\x0d\xfe\xed"
    struct.pack_into(">IIIIII", dtb, 4, 40, 40, 40, 40, 17, 16)
    return bytes(dtb)


class Lz4Tests(unittest.TestCase):
    def test_raw_lz4_roundtrips_small_random_and_repetitive_blocks(self):
        rng = random.Random(71)
        samples = [b"", b"a", b"abcd", b"a" * 64, bytes(range(256))]
        samples += [bytes(rng.randrange(256) for _ in range(n))
                    for n in (5, 12, 13, 31, 255, 4096, 65535)]
        samples += [b"abcdef0123456789" * 7000, b"a" * 100_000]
        for data in samples:
            packed = boot.lz4_compress_block(data)
            self.assertEqual(boot.lz4_decompress_block(packed, len(data)), data)

    def test_bol4_frame_lengths_fnv_and_payload(self):
        kernel = fake_kernel()
        frame = boot.build_bol4(kernel)
        self.assertEqual(frame[:4], b"BOL4")
        self.assertEqual(boot.unpack_bol4(frame), kernel)
        packed_len, output_len, checksum = struct.unpack_from("<III", frame, 4)
        self.assertEqual(packed_len, len(frame) - 16)
        self.assertEqual(output_len, len(kernel))
        self.assertEqual(checksum, boot.fnv1a32(frame[16:]))
        self.assertLess(packed_len, output_len)

    def test_bol4_rejects_corruption_and_wrong_lengths(self):
        frame = bytearray(boot.build_bol4(fake_kernel()))
        frame[16] ^= 1
        with self.assertRaisesRegex(boot.ImageError, "FNV"):
            boot.unpack_bol4(bytes(frame))
        frame = bytearray(boot.build_bol4(fake_kernel()))
        struct.pack_into("<I", frame, 4, len(frame))
        with self.assertRaisesRegex(boot.ImageError, "packed length"):
            boot.unpack_bol4(bytes(frame))
        with self.assertRaisesRegex(boot.ImageError, "capacity"):
            boot.unpack_bol4(boot.build_bol4(fake_kernel()), max_output=1024)

    def test_bol4_rejects_non_arm64_image(self):
        kernel = bytearray(fake_kernel())
        kernel[56:60] = b"NOPE"
        with self.assertRaisesRegex(boot.ImageError, "AArch64"):
            boot.build_bol4(bytes(kernel))

    def test_lz4_decoder_rejects_invalid_offset_and_truncated_extensions(self):
        with self.assertRaisesRegex(boot.ImageError, "offset"):
            boot.lz4_decompress_block(b"\x00\x00\x00", 4)
        with self.assertRaisesRegex(boot.ImageError, "extension"):
            boot.lz4_decompress_block(b"\xf0", 20)
        with self.assertRaisesRegex(boot.ImageError, "overruns"):
            boot.lz4_decompress_block(b"\x40abcd", 3)


class AndroidV2Tests(unittest.TestCase):
    def test_android_v2_pack_split_and_nested_bol4_roundtrip(self):
        header = fake_header()
        kernel = fake_kernel()
        bol4 = boot.build_bol4(kernel)
        dtb = fake_dtb()
        initrd = b"newc-initrd" * 17
        command = b"loglevel=1 console=ttyS0,115200"
        image = boot.build_android_v2(header, bol4, dtb, initrd, command)
        parsed_header, parts = boot.split_android_v2(image)
        self.assertEqual(parsed_header[0x40:0x40 + len(command)], command)
        self.assertEqual(parts, (bol4, initrd, b"", b"", dtb))
        self.assertEqual(boot.unpack_bol4(parts[0]), kernel)
        self.assertEqual(image[0x240:0x260], boot.android_image_id(parts))

    def test_android_v2_rejects_bad_id_trailer_and_dtb(self):
        image = bytearray(boot.build_android_v2(fake_header(), boot.build_bol4(fake_kernel()), fake_dtb()))
        image[0x240] ^= 1
        with self.assertRaisesRegex(boot.ImageError, "image ID"):
            boot.split_android_v2(bytes(image))
        image = boot.build_android_v2(fake_header(), boot.build_bol4(fake_kernel()), fake_dtb())
        with self.assertRaisesRegex(boot.ImageError, "trailing"):
            boot.split_android_v2(image + b"x")
        with self.assertRaisesRegex(boot.ImageError, "DTB"):
            boot.build_android_v2(fake_header(), boot.build_bol4(fake_kernel()), b"bad")

    def test_android_v2_refuses_unverified_load_addresses(self):
        header = bytearray(fake_header())
        struct.pack_into("<I", header, 12, 0x40000000)
        with self.assertRaisesRegex(boot.ImageError, "load addresses"):
            boot.build_android_v2(bytes(header), boot.build_bol4(fake_kernel()), fake_dtb())

    def test_android_v2_enforces_image_size_budget(self):
        with self.assertRaisesRegex(boot.ImageError, "16 MiB"):
            boot.build_android_v2(fake_header(), boot.build_bol4(fake_kernel()), fake_dtb(),
                                  max_image=4096)



    def test_bol4_decodes_with_pinned_baseos_native_c_decoder(self):
        compiler = shutil.which("cc") or shutil.which("clang")
        if compiler is None:
            self.skipTest("host C compiler is not installed")
        vendor = Path(__file__).parent / "vendor" / "baseos-native-lz4"
        harness = vendor / "decode_harness.c"
        decoder = vendor / "lz4-stream.c"
        header = vendor / "lz4-stream.h"

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            executable = root / ("decode.exe" if __import__("os").name == "nt" else "decode")
            frame_path = root / "kernel.BOL4"
            output_path = root / "kernel.Image"
            subprocess.run(
                [compiler, "-std=c11", "-O2", "-I", str(vendor), str(harness),
                 str(decoder), "-o", str(executable)],
                check=True, capture_output=True, text=True
            )

            kernel = fake_kernel()
            frame = boot.build_bol4(kernel)
            frame_path.write_bytes(frame)
            subprocess.run([str(executable), str(frame_path), str(output_path)],
                           check=True, capture_output=True, text=True)
            self.assertEqual(output_path.read_bytes(), kernel)

            damaged = bytearray(frame)
            damaged[12] ^= 1
            frame_path.write_bytes(damaged)
            rejected = subprocess.run([str(executable), str(frame_path), str(output_path)],
                                      capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)


    def test_audited_rg34xx_active_extent_rejects_structurally_valid_other_image(self):
        kernel = fake_kernel()
        bol4 = boot.build_bol4(kernel)
        dtb_size = (boot.RG34XX_ACTIVE_IMAGE_BYTES - boot.PAGE_SIZE -
                    boot.align_page(len(bol4)))
        dtb = bytearray(dtb_size)
        dtb[:4] = b"\xd0\x0d\xfe\xed"
        struct.pack_into(">I", dtb, 4, 40)
        image = boot.build_android_v2(fake_header(), bol4, bytes(dtb),
                                      max_image=boot.MAX_ANDROID_IMAGE)
        self.assertEqual(len(image), boot.RG34XX_ACTIVE_IMAGE_BYTES)
        with self.assertRaisesRegex(boot.ImageError, "FNV-1a mismatch"):
            boot.validate_rg34xx_reference_active_image(image, bol4)

    def test_extract_inputs_from_local_partition_dump(self):
        kernel = fake_kernel()
        bol4 = boot.build_bol4(kernel)
        initrd = b"newc-empty-initrd"
        dtb = fake_dtb()
        image = boot.build_android_v2(fake_header(), bol4, dtb, initrd)
        offset = 0x03000000
        partition = bytearray(offset + len(image) + 512)
        partition[offset:offset + len(image)] = image

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            partition_path = root / "boot-partition.bin"
            output_dir = root / "extracted"
            partition_path.write_bytes(partition)
            source_before = partition_path.read_bytes()
            output_dir.mkdir()

            manifest = boot.write_extracted_inputs(partition_path, offset, len(image), output_dir)

            self.assertEqual(partition_path.read_bytes(), source_before)
            self.assertTrue(manifest["host_only"])
            self.assertTrue(manifest["not_installable"])
            self.assertEqual((output_dir / "active-boot.img").read_bytes(), image)
            self.assertEqual((output_dir / "android-v2-header.bin").read_bytes(), image[:boot.PAGE_SIZE])
            self.assertEqual((output_dir / "kernel.BOL4").read_bytes(), bol4)
            self.assertEqual((output_dir / "kernel.Image").read_bytes(), kernel)
            self.assertEqual((output_dir / "initrd.img").read_bytes(), initrd)
            self.assertEqual((output_dir / "dtb.bin").read_bytes(), dtb)
            saved_manifest = json.loads((output_dir / "extracted-manifest.json").read_text())
            self.assertEqual(saved_manifest["source_image_sha256"], manifest["source_image_sha256"])

            with self.assertRaisesRegex(boot.ImageError, "must be empty"):
                boot.write_extracted_inputs(partition_path, offset, len(image), output_dir)

    def test_extract_inputs_rejects_out_of_bounds_extent(self):
        with self.assertRaisesRegex(boot.ImageError, "outside"):
            boot.extract_android_v2_inputs(b"\0" * 4096, 0x3000, 2048)

    def test_cli_build_and_inspect_are_file_only_roundtrip(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            kernel = root / "Image"
            header = root / "header.bin"
            dtb = root / "board.dtb"
            output = root / "boot.img"
            kernel.write_bytes(fake_kernel())
            header.write_bytes(fake_header())
            dtb.write_bytes(fake_dtb())
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                self.assertEqual(boot.main(["build", "--kernel", str(kernel),
                                            "--header", str(header), "--dtb", str(dtb),
                                            "--output", str(output)]), 0)
            manifest = json.loads(stdout.getvalue())
            self.assertTrue(manifest["host_only"])
            self.assertTrue(manifest["not_installable"])
            self.assertTrue(manifest["requires_matching_native_firmware_rebuild"])
            self.assertTrue(output.is_file())
            self.assertTrue((root / "boot.img.bol4").is_file())
            self.assertTrue((root / "boot.img.json").is_file())
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                self.assertEqual(boot.main(["inspect", str(output)]), 0)
            self.assertEqual(json.loads(stdout.getvalue())["kernel_bytes"], len(fake_kernel()))


class NativeFirmwareTests(unittest.TestCase):
    def fake_firmware(self):
        monitor_payload = struct.pack("<I", 0x1400003E) + b"M" * (0x1A2D0 - 4)
        monitor = monitor_payload + bytes(0x1A400 - len(monitor_payload))
        dtbo_payload = bytearray(0x920)
        dtbo_payload[:4] = b"\xd0\x0d\xfe\xed"
        struct.pack_into(">I", dtbo_payload, 4, 0x920)
        dtbo = bytes(dtbo_payload) + bytes(0xA00 - len(dtbo_payload))
        dtb = bytearray(0x23400)
        dtb[:4] = b"\xd0\x0d\xfe\xed"
        struct.pack_into(">I", dtb, 4, 40)
        loader = struct.pack("<I", 0xEA0003FE) + b"uboot" + bytes(0x8000 - 9)
        return monitor + dtbo + bytes(dtb) + loader

    def test_native_firmware_extents_and_loader_header_are_bounded(self):
        firmware = self.fake_firmware()
        parts = boot.split_rg34xx_native_firmware(
            firmware, expected_sha256=None, expected_fnv=None
        )
        self.assertEqual(len(parts["firmware-package.bin"]), boot.RG34XX_FIRMWARE_BYTES)
        self.assertEqual(len(parts["monitor.bin"]), 0x1A2D0)
        self.assertEqual(len(parts["dtbo.bin"]), 0x920)
        self.assertEqual(len(parts["dtb.bin"]), 0x23400)
        self.assertEqual(len(parts["loader.bin"]), 0x8000)
        self.assertEqual(parts["loader.bin"][:4], struct.pack("<I", 0xEA0003FE))

        damaged = bytearray(firmware)
        damaged[0x1A3FF] = 1
        with self.assertRaisesRegex(boot.ImageError, "padding"):
            boot.split_rg34xx_native_firmware(
                bytes(damaged), expected_sha256=None, expected_fnv=None
            )

        bad_dtbo = bytearray(firmware)
        bad_dtbo[0x1A400] = 0
        with self.assertRaisesRegex(boot.ImageError, "DTBO has no FDT header"):
            boot.split_rg34xx_native_firmware(
                bytes(bad_dtbo), expected_sha256=None, expected_fnv=None
            )

        bad_dtb_padding = bytearray(firmware)
        bad_dtb_padding[0x1AE00 + 40] = 1
        with self.assertRaisesRegex(boot.ImageError, "DTB trailing padding"):
            boot.split_rg34xx_native_firmware(
                bytes(bad_dtb_padding), expected_sha256=None, expected_fnv=None
            )

    def test_native_firmware_reference_fingerprint_rejects_other_package(self):
        with self.assertRaisesRegex(boot.ImageError, "SHA-256"):
            boot.split_rg34xx_native_firmware(self.fake_firmware())

    def test_native_firmware_rejects_wrong_loader_entry(self):
        firmware = bytearray(self.fake_firmware())
        firmware[0x3E200] ^= 1
        with self.assertRaisesRegex(boot.ImageError, "loader entry/header"):
            boot.split_rg34xx_native_firmware(
                bytes(firmware), expected_sha256=None, expected_fnv=None
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
