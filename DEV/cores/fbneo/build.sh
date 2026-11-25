#!/bin/sh
# FBNeo（libretro）—— 设备构建配方。
#
# 与 snes9x/fceumm 同一套约定：stamp 输出可比较的构建指纹，build 编译并写
# `.meta`。核心以 LSP 平台包分发（不随卡包），因此 manifest 里标 optional。
#
# 设备基线是 Debian 11 bullseye / glibc 2.31。aarch64 主机直接编译；其他主机
# 用 bullseye 的 aarch64 交叉工具链，产出的仍是面向 glibc 2.31 的 ELF（交叉
# 只影响编译速度，不改变 ABI 目标）。
set -eu

here="$(cd "$(dirname "$0")" && pwd)"

usage() {
	echo "usage: $0 stamp COMMIT | build COMMIT TARBALL WORKDIR OUT" >&2
	exit 2
}

sha256() {
	if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi |
		cut -d' ' -f1
}

stamp() {
	echo "commit=$1"
	echo "source=https://github.com/libretro/FBNeo/tree/$1"
	echo "recipe=make -C src/burner/libretro platform=unix"
	# 全量驱动集（SUBSET=all，默认）：一份 .so 同时服务 Arcade 与 NeoGeo。
	echo "subset=all"
}

install_deps() {
	if command -v g++ >/dev/null 2>&1 && [ "$(uname -m)" = "aarch64" ]; then
		return 0
	fi
	command -v apt-get >/dev/null 2>&1 || {
		echo "building FBNeo needs g++/make: install them first" >&2
		exit 1
	}
	if command -v aarch64-linux-gnu-g++ >/dev/null 2>&1; then
		return 0
	fi
	echo "installing the aarch64 cross toolchain" >&2
	sed -i '/bullseye-security/d' /etc/apt/sources.list 2>/dev/null || true
	apt-get -o Acquire::Check-Valid-Until=false update -qq
	apt-get install -y -qq --no-install-recommends \
		g++-aarch64-linux-gnu gcc-aarch64-linux-gnu binutils-aarch64-linux-gnu \
		make ca-certificates >/dev/null
}

build() {
	commit="$1" tarball="$2" work="$3" out="$4"
	src="$work/FBNeo-$commit"

	install_deps
	rm -rf "$work"
	mkdir -p "$work"
	tar -xzf "$tarball" -C "$work"
	if [ ! -f "$src/src/burner/libretro/Makefile" ]; then
		echo "$tarball does not hold FBNeo-$commit/src/burner/libretro/Makefile" >&2
		exit 1
	fi

	if [ "$(uname -m)" = "aarch64" ]; then
		cc=cc
		cxx=c++
		strip_tool=strip
	else
		cc=aarch64-linux-gnu-gcc
		cxx=aarch64-linux-gnu-g++
		strip_tool=aarch64-linux-gnu-strip
	fi

	jobs=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)
	make -C "$src/src/burner/libretro" platform=unix -j"$jobs" CC="$cc" CXX="$cxx"

	mkdir -p "$(dirname "$out")"
	cp "$src/src/burner/libretro/fbneo_libretro.so" "$out"
	"$strip_tool" --strip-unneeded "$out"
	stamp "$commit" >"$out.meta"
}

case "${1:-}" in
stamp)
	[ $# -eq 2 ] && [ -n "$2" ] || usage
	stamp "$2"
	;;
build)
	[ $# -eq 5 ] && [ -n "$2" ] && [ -n "$3" ] && [ -n "$4" ] && [ -n "$5" ] || usage
	build "$2" "$3" "$4" "$5"
	;;
*)
	usage
	;;
esac
