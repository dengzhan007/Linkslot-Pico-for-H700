#!/bin/sh
# armGBA libretro core (GPL-2.0, includes Gambatte).
#
# Delivered as an optional LSP pack rather than the base image, so this script
# is not part of build/fetch-cores.sh yet.  It follows the same stamp/build
# contract as cores/mgba/build.sh so it can be wired into the manifest later.
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
source_url="https://github.com/MagnominusGit/armGBA"

usage() {
	echo "usage: $0 stamp COMMIT | build COMMIT WORKDIR OUT" >&2
	exit 2
}

stamp() {
	echo "commit=$1"
	echo "source=$source_url/tree/$1"
	echo "recipe=cmake -DARMGBA_SDL=OFF -DARMGBA_JIT=ON"
}

install_deps() {
	need=""
	command -v cmake >/dev/null 2>&1 || need="$need cmake"
	command -v g++ >/dev/null 2>&1 || need="$need g++"
	[ -f /usr/include/zlib.h ] || need="$need zlib1g-dev"
	[ -z "$need" ] && return 0
	command -v apt-get >/dev/null 2>&1 || {
		echo "building armGBA needs:$need" >&2
		exit 1
	}
	sed -i '/bullseye-security/d' /etc/apt/sources.list 2>/dev/null || true
	apt-get -o Acquire::Check-Valid-Until=false update -qq
	# shellcheck disable=SC2086
	apt-get install -y -qq --no-install-recommends $need >/dev/null
}

build() {
	commit="$1" work="$2" out="$3"
	src="$work/armGBA"
	obj="$work/build"

	mkdir -p "$work"
	if [ ! -d "$src/.git" ] && [ -f "$src/CMakeLists.txt" ]; then
		# Offline/pre-cloned source tree: build it as it is.  Useful on hosts
		# whose Docker network cannot reach GitHub (the stamp still records
		# the pinned commit).
		echo "using the pre-placed source tree at $src" >&2
	elif [ -d "$src/.git" ]; then
		git -C "$src" cat-file -e "$commit^{commit}" 2>/dev/null ||
			git -C "$src" fetch -q --depth 1 "$source_url" "$commit"
		git -C "$src" checkout -q --force --detach "$commit"
		git -C "$src" clean -q -fdx
	else
		mkdir -p "$src"
		git init -q "$src"
		git -C "$src" fetch -q --depth 1 "$source_url" "$commit"
		git -C "$src" checkout -q --force --detach "$commit"
	fi

	install_deps

	cmake -S "$src" -B "$obj" \
		-DCMAKE_BUILD_TYPE=Release \
		-DARMGBA_SDL=OFF \
		-DARMGBA_JIT=ON >/dev/null
	cmake --build "$obj" --target armgba_libretro \
		--parallel "$(getconf _NPROCESSORS_ONLN)" >/dev/null

	mkdir -p "$(dirname "$out")"
	cp "$obj/armgba_libretro.so" "$out"
	stamp "$commit" >"$out.meta"

	# GPL-2.0 corresponding source (the core does not use the armulators
	# submodule: ARMGBA_SDL=OFF).
	if [ -d "$src/.git" ]; then
		git -C "$src" archive --format=tar.gz \
			--prefix="armGBA-$commit/" -o "$work/armGBA-$commit.tar.gz" HEAD
	else
		tar -czf "$work/armGBA-$commit.tar.gz" --exclude=.git \
			-C "$src" .
	fi
	echo "source tarball: $work/armGBA-$commit.tar.gz"
}

case "${1:-}" in
stamp)
	[ $# -eq 2 ] && [ -n "$2" ] || usage
	stamp "$2"
	;;
build)
	[ $# -eq 4 ] && [ -n "$2" ] && [ -n "$3" ] && [ -n "$4" ] || usage
	build "$2" "$3" "$4"
	;;
*)
	usage
	;;
esac
