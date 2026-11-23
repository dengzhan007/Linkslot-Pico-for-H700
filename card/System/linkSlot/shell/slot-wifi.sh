#!/bin/sh
# linkSlot 设备端 Wi-Fi 控制脚本（BaseOS：wpa_supplicant + udhcpc）。
#
# 用法：
#   slot-wifi.sh status            # stdout: state=connected|disconnected / ssid= / ip=
#   slot-wifi.sh scan              # stdout: 每行 SSID<TAB>信号<TAB>加密(1/0)
#   slot-wifi.sh radio-status      # stdout: radio=on|off（Wi-Fi 无线电是否开启）
#   slot-wifi.sh radio on|off      # 开/关 Wi-Fi 无线电（rfkill + ip link），stdout: radio=on|off
#   slot-wifi.sh connect SSID PSK  # 委托 System/linkSlot/shell/linkslot-net.sh up 联网（空 PSK=开放网络）
#
# stdout 是给 slot 前端的机器可读契约；进度/错误只写 stderr（中文）。
# status/radio-status 永远 exit 0；scan/connect/radio on|off 失败返回非 0。
# radio off 后 scan 直接报错（不会去启动临时 wpa_supplicant）。
# connect 不复制关联逻辑：设置 LINKSLOT_WIFI_* 后 exec linkslot-net.sh up，
# 由它负责 rfkill/关联/DHCP/超时。
#
# 可用环境变量覆盖（便于测试/无命令的极简系统，风格同 linkslot-net.sh）：
#   AGS_LINKSLOT_WLAN=wlan0                无线网卡名
#   AGS_WPA_CLI=wpa_cli                    wpa_cli 路径
#   AGS_WPA_SUPPLICANT=wpa_supplicant       wpa_supplicant 路径
#   AGS_IP=ip                              ip 命令路径
#   AGS_RFKILL=rfkill                      rfkill 命令路径（radio-status / radio on|off）
#   AGS_CTRL_DIR=/var/run/wpa_supplicant   常驻 supplicant 控制目录
#   AGS_SLOT_WIFI_CTRL=/run/slot-wifi      临时扫描控制目录（扫描后清理）
#   AGS_SLOT_WIFI_CONF=$CTRL/wpa.conf      临时 wpa_supplicant 配置
#   AGS_SLOT_WIFI_SCAN_WAIT_S=40           等待扫描结果的最多轮数（每轮 0.25s）
#   AGS_SLOT_WIFI_NET_SCRIPT=...           connect 委托的联网脚本（默认 <卡根>/System/linkSlot/shell/linkslot-net.sh）
#   LINKSLOT_WIFI_TIMEOUT / LINKSLOT_WIFI_SSID / LINKSLOT_WIFI_PSK  传给 linkslot-net.sh
set -u

SCRIPT_DIR=$(dirname "$0")
# 脚本位于 <卡根>/System/linkSlot/shell/：向上三级才是卡根。
# （重构前脚本在 <卡根>/System/shell/，只向上两级——这正是 Wi-Fi 连不上的原因。）
ROOT=$(CDPATH= cd "$SCRIPT_DIR/../../.." 2>/dev/null && pwd) || ROOT=.
if [ -n "${SLOT_ROOT:-}" ]; then
	ROOT="$SLOT_ROOT"
fi
if [ -n "${AGS_SLOT_WIFI_ROOT:-}" ]; then
	ROOT="$AGS_SLOT_WIFI_ROOT"
fi

# 排序/去重只依赖数字键，C locale 可避免不同设备的 collation 差异。
LC_ALL=C
export LC_ALL

WLAN="${AGS_LINKSLOT_WLAN:-wlan0}"
WPA_CLI="${AGS_WPA_CLI:-wpa_cli}"
WPA_SUPPLICANT="${AGS_WPA_SUPPLICANT:-wpa_supplicant}"
IP="${AGS_IP:-ip}"
RFKILL="${AGS_RFKILL:-rfkill}"
CTRL_DIR="${AGS_CTRL_DIR:-/var/run/wpa_supplicant}"
TMP_CTRL="${AGS_SLOT_WIFI_CTRL:-/run/slot-wifi}"
TMP_CONF="${AGS_SLOT_WIFI_CONF:-$TMP_CTRL/wpa.conf}"
NET_SCRIPT="${AGS_SLOT_WIFI_NET_SCRIPT:-$ROOT/System/linkSlot/shell/linkslot-net.sh}"

SCAN_WAIT="${AGS_SLOT_WIFI_SCAN_WAIT_S:-40}"
case "$SCAN_WAIT" in
'' | *[!0-9]*) SCAN_WAIT=40 ;;
esac

TAB=$(printf '\t')
SCAN_TEMP=0
LOG_LEVEL="${LINKSLOT_LOG_LEVEL:-info}"

log() {
	echo "slot-wifi: $*" >&2
}

debug() {
	[ "$LOG_LEVEL" = debug ] && echo "[debug] slot-wifi: $*" >&2
}

sleep_short() {
	sleep 0.25 2>/dev/null || sleep 1
}

# --- radio ----------------------------------------------------------------

# True (exit 0) 表示 Wi-Fi 无线电已开启。
# 优先 `rfkill list wifi`：任一 Soft/Hard blocked yes → off；有 wifi 设备且
# 未 blocked → on。rfkill 不存在/不识别 wifi 类型时退化为 `ip link show`
# 是否存在且处于 UP（无网卡则 off）。
radio_state() {
	if command -v "$RFKILL" >/dev/null 2>&1; then
		rfkill_out=$("$RFKILL" list wifi 2>/dev/null) || rfkill_out=
		if [ -n "$rfkill_out" ]; then
			case "$rfkill_out" in
			*"Soft blocked: yes"* | *"Hard blocked: yes"*) return 1 ;;
			esac
			return 0
		fi
	fi
	link_out=$("$IP" link show "$WLAN" 2>/dev/null) || return 1
	case "$link_out" in
	*"<UP,"* | *",UP,"* | *",UP>"* | *"<UP>"*) return 0 ;;
	esac
	return 1
}

do_radio_status() {
	debug "radio status requested"
	if radio_state; then
		printf 'radio=on\n'
	else
		printf 'radio=off\n'
	fi
	return 0
}

do_radio_on() {
	debug "radio on: before rfkill/link state"
	if command -v "$RFKILL" >/dev/null 2>&1; then
		"$RFKILL" unblock wifi >/dev/null 2>&1 || true
	fi
	"$IP" link set "$WLAN" up >/dev/null 2>&1 || true
	# rfkill/驱动状态切换有延迟，最多等约 2s。
	i=0
	while ! radio_state && [ "$i" -lt 8 ]; do
		sleep_short
		i=$((i + 1))
	done
	if radio_state; then
		printf 'radio=on\n'
		log "Wi-Fi 无线电已开启"
		return 0
	fi
	log "开启 WiFi 失败（请检查 rfkill 软件/硬件开关）"
	return 1
}

do_radio_off() {
	debug "radio off: before rfkill/link state"
	"$IP" link set "$WLAN" down >/dev/null 2>&1 || true
	"$WPA_CLI" -p "$CTRL_DIR" -i "$WLAN" terminate >/dev/null 2>&1 || true
	if command -v "$RFKILL" >/dev/null 2>&1; then
		"$RFKILL" block wifi >/dev/null 2>&1 || true
	fi
	# 重复关闭是成功路径（无线电已经关闭时立即返回）。
	i=0
	while radio_state && [ "$i" -lt 8 ]; do
		sleep_short
		i=$((i + 1))
	done
	if radio_state; then
		log "关闭 WiFi 失败（rfkill 不可用或无线网卡仍在）"
		return 1
	fi
	printf 'radio=off\n'
	log "Wi-Fi 无线电已关闭"
	return 0
}

# --- status ---------------------------------------------------------------

wpa_state() {
	"$WPA_CLI" -p "$CTRL_DIR" -i "$WLAN" status 2>/dev/null |
		sed -n 's/^wpa_state=//p' | head -n 1
}

wpa_ssid() {
	"$WPA_CLI" -p "$CTRL_DIR" -i "$WLAN" status 2>/dev/null |
		sed -n 's/^ssid=//p' | head -n 1
}

wpa_ip() {
	"$WPA_CLI" -p "$CTRL_DIR" -i "$WLAN" status 2>/dev/null |
		sed -n 's/^ip_address=//p' | head -n 1
}

# 输出第一个可用的 IPv4 地址（排除 127/8 与 169.254/16）。
# 与 linkslot-net.sh 一致：不用 -o/-4，BaseOS 的 busybox ip applet 未必实现。
wlan_ipv4() {
	ips=$("$IP" addr show dev "$WLAN" 2>/dev/null |
		sed -n 's/.*[[:space:]]inet[[:space:]]\([0-9][0-9.]*\)\/.*/\1/p')
	[ -n "$ips" ] || return 1
	for ip in $ips; do
		case "$ip" in
		127.* | 169.254.*) ;;
		*)
			echo "$ip"
			return 0
			;;
		esac
	done
	return 1
}

do_status() {
	debug "status: wpa_state=$(wpa_state || true) ssid=$(wpa_ssid || true) ip=$(wlan_ipv4 || true) rfkill=$(radio_state && echo on || echo off)"
	state=$(wpa_state || true)
	ssid=$(wpa_ssid || true)
	ip=$(wlan_ipv4 || true)
	[ -n "$ip" ] || ip=$(wpa_ip || true)
	if [ "$state" = COMPLETED ] && [ -n "$ip" ]; then
		printf 'state=connected\nssid=%s\nip=%s\n' "$ssid" "$ip"
		log "已连接：ssid=$ssid ip=$ip"
	else
		printf 'state=disconnected\nssid=\nip=\n'
		log "未连接：wpa_state=${state:-unknown}"
	fi
	return 0
}

# --- scan -----------------------------------------------------------------

cleanup_scan() {
	[ "$SCAN_TEMP" = 1 ] || return 0
	"$WPA_CLI" -p "$TMP_CTRL" -i "$WLAN" terminate >/dev/null 2>&1 || true
	rm -rf "$TMP_CTRL" 2>/dev/null || true
	SCAN_TEMP=0
	return 0
}

start_temp_supplicant() {
	command -v "$WPA_SUPPLICANT" >/dev/null 2>&1 || {
		log "缺少 $WPA_SUPPLICANT，无法扫描 Wi-Fi"
		return 1
	}
	command -v "$WPA_CLI" >/dev/null 2>&1 || {
		log "缺少 $WPA_CLI，无法读取 Wi-Fi 扫描结果"
		return 1
	}
	if ! mkdir -p "$TMP_CTRL" 2>/dev/null; then
		log "无法创建临时目录 $TMP_CTRL"
		return 1
	fi
	rm -f "$TMP_CONF" 2>/dev/null || true
	(
		umask 077
		{
			echo "ctrl_interface=$TMP_CTRL"
			echo 'update_config=0'
			echo 'ap_scan=1'
		} >"$TMP_CONF"
	) || {
		log "无法写入临时配置 $TMP_CONF"
		rm -rf "$TMP_CTRL" 2>/dev/null || true
		return 1
	}
	"$WPA_SUPPLICANT" -B -i "$WLAN" -c "$TMP_CONF" -Dnl80211 >/dev/null 2>&1 ||
		"$WPA_SUPPLICANT" -B -i "$WLAN" -c "$TMP_CONF" >/dev/null 2>&1 || {
			log "启动 wpa_supplicant 扫描失败"
			rm -rf "$TMP_CTRL" 2>/dev/null || true
			return 1
		}
	SCAN_TEMP=1
	# 等控制 socket 出现（最多约 5s）。
	i=0
	while [ ! -e "$TMP_CTRL/$WLAN" ] && [ "$i" -lt 20 ]; do
		sleep_short
		i=$((i + 1))
	done
	if [ ! -e "$TMP_CTRL/$WLAN" ]; then
		log "wpa_supplicant 未创建控制接口"
		cleanup_scan
		return 1
	fi
	return 0
}

# 把 wpa_cli scan_results 的制表符分隔行转成 stdout 契约：
# 去掉表头、过滤空/隐藏 SSID、SSID 内 TAB 换成空格、标记是否加密，
# 然后按信号降序排序、按 SSID 去重（保留最强的那个）。
# 只用 BusyBox 一定带的 sed/sort/grep/tr，不依赖 awk。
parse_scan_results() {
	# 去重用的已见 SSID 清单（grep -F 逐行精确匹配）。
	# 设备上 /run 通常是可写 tmpfs，/tmp 作为后备；都不可写时退化为不去重。
	seen_file=
	for dir in "${TMPDIR:-}" /run /tmp; do
		[ -n "$dir" ] || continue
		[ -d "$dir" ] || continue
		if command -v mktemp >/dev/null 2>&1; then
			seen_file=$(mktemp "$dir/slot-wifi-seen.XXXXXX" 2>/dev/null) && break
			seen_file=
		fi
		if : >"$dir/slot-wifi-seen.$$" 2>/dev/null; then
			seen_file="$dir/slot-wifi-seen.$$"
			break
		fi
	done
	printf '%s\n' "$1" |
		sed -e '1d' -e 's/\r$//' |
		while IFS="$TAB" read -r bssid freq signal flags ssid; do
			[ -n "${ssid:-}" ] || continue
			case "$signal" in
			'' | *[!0-9-]*) continue ;;
			esac
			ssid=$(printf '%s' "$ssid" | tr '\t' ' ')
			case "$flags" in
			*WPA* | *RSN* | *WEP* | *SAE* | *OWE* | *EAP* | *802.1X*)
				secured=1
				;;
			*) secured=0 ;;
			esac
			printf '%s\t%s\t%s\n' "$ssid" "$signal" "$secured"
		done |
		sort -t "$TAB" -k2,2nr -k1,1 |
		while IFS= read -r line; do
			ssid=${line%%"$TAB"*}
			if [ -n "$seen_file" ] && grep -qxF -e "$ssid" "$seen_file" 2>/dev/null; then
				continue
			fi
			if [ -n "$seen_file" ]; then
				printf '%s\n' "$ssid" >>"$seen_file" 2>/dev/null || true
			fi
			printf '%s\n' "$line"
		done
	if [ -n "$seen_file" ]; then
		rm -f "$seen_file" 2>/dev/null || true
	fi
}

do_scan() {
	SCAN_TEMP=0
	# 无线电关闭时没必要（也不应该）启动临时 supplicant。
	if ! radio_state; then
		log "WiFi 已关闭，无法扫描网络"
		return 1
	fi
	debug "scan: radio on ctrl=$CTRL_DIR"
	ctrl="$CTRL_DIR"
	if [ ! -e "$CTRL_DIR/$WLAN" ]; then
		start_temp_supplicant || return 1
		ctrl="$TMP_CTRL"
	fi
	# 中途被杀时尽量清理临时 supplicant（扫描本身只读，不动常驻配置）。
	trap 'cleanup_scan' EXIT HUP INT TERM

	"$WPA_CLI" -p "$ctrl" -i "$WLAN" scan >/dev/null 2>&1 || true
	results=
	i=0
	while :; do
		if results=$("$WPA_CLI" -p "$ctrl" -i "$WLAN" scan_results 2>/dev/null); then
			# scan_results prints its bssid header even before the scan has
			# produced any AP.  Wait for at least one data row, otherwise a
			# first-time scan (no cached results) looks like "no networks".
			if [ -n "$results" ] &&
				printf '%s\n' "$results" | head -n 1 | grep -q '^bssid' &&
				printf '%s\n' "$results" | sed -n '2p' | grep -q '[^[:space:]]'; then
				break
			fi
		fi
		[ "$i" -ge "$SCAN_WAIT" ] && break
		sleep_short
		i=$((i + 1))
	done
	if [ -z "$results" ]; then
		log "读取 Wi-Fi 扫描结果失败"
		cleanup_scan
		return 1
	fi

	parse_scan_results "$results"
	rc=$?
	cleanup_scan
	return "$rc"
}

# --- connect --------------------------------------------------------------

do_connect() {
	ssid="${1:-}"
	psk="${2:-}"
	if [ -z "$ssid" ]; then
		log "用法：slot-wifi.sh connect SSID [PSK]"
		return 2
	fi
	if [ ! -f "$NET_SCRIPT" ]; then
		log "缺少联网脚本 $NET_SCRIPT，无法连接 Wi-Fi"
		return 1
	fi
	LINKSLOT_WIFI_SSID="$ssid"
	LINKSLOT_WIFI_PSK="$psk"
	LINKSLOT_WIFI_TIMEOUT="${LINKSLOT_WIFI_TIMEOUT:-45}"
	export LINKSLOT_WIFI_SSID LINKSLOT_WIFI_PSK LINKSLOT_WIFI_TIMEOUT
	debug "connect: ssid=$ssid psk_set=$([ -n "$psk" ] && echo yes || echo no) net_script=$NET_SCRIPT"
	log "正在连接 Wi-Fi：$ssid"
	exec /bin/sh "$NET_SCRIPT" up
}

case "${1:-}" in
status) do_status ;;
scan) do_scan ;;
radio-status) do_radio_status ;;
radio)
	shift
	case "${1:-}" in
	on) do_radio_on ;;
	off) do_radio_off ;;
	*)
		log "用法：slot-wifi.sh radio on|off"
		exit 2
		;;
	esac
	;;
connect)
	shift
	do_connect "$@"
	;;
*)
	echo "usage: slot-wifi.sh status|scan|radio-status|radio on|off|connect SSID [PSK]" >&2
	exit 2
	;;
esac
