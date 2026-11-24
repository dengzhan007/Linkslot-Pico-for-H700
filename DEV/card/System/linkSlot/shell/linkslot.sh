#!/bin/sh
# linkSlot 的启动包装：BaseOS 的前端卡是 FAT32，二进制可能没有可执行位，
# 因此和 System/linkSlot/bin/linkslot 一样，经动态 loader 启动而不是直接 exec。
# Slot 通过 /bin/sh 调用本脚本，脚本不需要可执行位。
set -u

ROOT="${SLOT_ROOT:-/mnt/sdcard}"
BIN="$ROOT/System/linkSlot/bin/linkslot_sync"

if [ ! -f "$BIN" ]; then
	echo "linkslot: $BIN not found" >&2
	exit 3
fi

export SLOT_ROOT="$ROOT"
exec /lib/ld-linux-aarch64.so.1 "$BIN" "$@"
