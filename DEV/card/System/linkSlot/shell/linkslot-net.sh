#!/bin/sh
# linkSlot 按需联网脚本（BaseOS：wpa_supplicant + udhcpc）。
#
# 用法：
#   linkslot-net.sh status    # 快速探测：wlan0 存在/UP/已关联/有 IPv4，成功 exit 0
#   linkslot-net.sh up        # 按需联网：rfkill → ip link up → 关联 → DHCP，成功 exit 0
#
# 所有进度/错误信息只写 stderr（人类可读）；stdout 不输出任何内容，
# 也不输出 LINKSLOT_* 行（stdout 契约由二进制的 CLI 负责）。
#
# 可用环境变量覆盖（便于测试/无命令的极简系统，风格同 slotlink.sh 的 AGS_*）：
#   AGS_NET_SYS=/sys/class/net        网络设备目录
#   AGS_RUN=/run                     运行目录（wpa_supplicant 控制 socket）
#   AGS_CTRL_DIR=/var/run/wpa_supplicant
#   AGS_RESOLV_CONF=$RUN/resolv.conf  DHCP 后等待的 resolv.conf
#   AGS_LINKSLOT_SSID / LINKSLOT_WIFI_SSID          Wi-Fi SSID（CLI 注入）
#   AGS_LINKSLOT_PSK  / LINKSLOT_WIFI_PSK           Wi-Fi 密码（CLI 注入）
#   AGS_LINKSLOT_TIMEOUT / LINKSLOT_WIFI_TIMEOUT    总超时秒数（默认 30）
#   AGS_LINKSLOT_ASSOC_WAIT_S=20                    等待关联完成秒数
#   AGS_LINKSLOT_DHCP_WAIT_S=10                     等待 IPv4 秒数
#   AGS_WPA_SUPPLICANT / AGS_WPA_CLI / AGS_IP / AGS_UDHCPC / AGS_RFKILL  命令路径
set -u

NET="${AGS_NET_SYS:-/sys/class/net}"
RUN="${AGS_RUN:-/run}"
CTRL_DIR="${AGS_CTRL_DIR:-/var/run/wpa_supplicant}"
RESOLV_CONF="${AGS_RESOLV_CONF:-$RUN/resolv.conf}"
WLAN="${AGS_LINKSLOT_WLAN:-wlan0}"

SSID="${AGS_LINKSLOT_SSID:-${LINKSLOT_WIFI_SSID:-}}"
PSK="${AGS_LINKSLOT_PSK:-${LINKSLOT_WIFI_PSK:-}}"
TIMEOUT="${AGS_LINKSLOT_TIMEOUT:-${LINKSLOT_WIFI_TIMEOUT:-45}}"
case "$TIMEOUT" in '' | *[!0-9]*) TIMEOUT=45 ;; esac

WPA_SUPPLICANT="${AGS_WPA_SUPPLICANT:-wpa_supplicant}"
WPA_CLI="${AGS_WPA_CLI:-wpa_cli}"
IP="${AGS_IP:-ip}"
UDHCPC="${AGS_UDHCPC:-udhcpc}"
RFKILL="${AGS_RFKILL:-rfkill}"

DHCP_WAIT="${AGS_LINKSLOT_DHCP_WAIT_S:-10}"
case "$DHCP_WAIT" in '' | *[!0-9]*) DHCP_WAIT=10 ;; esac

# Association budget: default 30s, but never more than the total timeout minus
# the DHCP budget (the CLI waits timeout+grace for the whole script).
if [ -n "${AGS_LINKSLOT_ASSOC_WAIT_S:-}" ]; then
	ASSOC_WAIT="$AGS_LINKSLOT_ASSOC_WAIT_S"
else
	ASSOC_WAIT=30
	budget=$((TIMEOUT - DHCP_WAIT))
	[ "$budget" -lt 30 ] && ASSOC_WAIT="$budget"
	[ "$ASSOC_WAIT" -lt 5 ] && ASSOC_WAIT=5
fi
case "$ASSOC_WAIT" in '' | *[!0-9]*) ASSOC_WAIT=30 ;; esac

WPA_CONF="$RUN/linkslot-wpa.conf"
LOG_LEVEL="${LINKSLOT_LOG_LEVEL:-info}"

log() {
	echo "linkslot-net: $*" >&2
}

debug() {
	[ "$LOG_LEVEL" = debug ] && echo "[debug] linkslot-net: $*" >&2
}

sleep_short() {
	sleep 0.25 2>/dev/null || sleep 1
}

wlan_exists() {
	[ -d "$NET/$WLAN" ]
}

# UP / dormant / unknown all mean "the interface is administratively up";
# association is checked separately through wpa_supplicant.
wlan_link_up() {
	[ -d "$NET/$WLAN" ] || return 1
	state=$(cat "$NET/$WLAN/operstate" 2>/dev/null || echo unknown)
	case "$state" in
	up | dormant | unknown) return 0 ;;
	*) return 1 ;;
	esac
}

supplicant_available() {
	command -v "$WPA_CLI" >/dev/null 2>&1
}

supplicant_state() {
	"$WPA_CLI" -p "$CTRL_DIR" -i "$WLAN" status 2>/dev/null |
		sed -n 's/^wpa_state=//p' | head -n 1
}

supplicant_ssid() {
	"$WPA_CLI" -p "$CTRL_DIR" -i "$WLAN" status 2>/dev/null |
		sed -n 's/^ssid=//p' | head -n 1
}

# Echo the first usable IPv4 address (link-local 169.254/16 excluded).
# Use plain `ip addr show` (no -o/-4) because the BaseOS busybox ip applet may
# not implement those flags; `inet6` lines cannot match the ` inet ` pattern.
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

wlan_default_route() {
	"$IP" route show default 2>/dev/null | grep -q '^default'
}

default_gateway() {
	"$IP" route show default 2>/dev/null |
		sed -n 's/.*via[[:space:]]\([0-9][0-9.]*\).*/\1/p' |
		head -n 1
}

resolv_ok() {
	[ -s "$RESOLV_CONF" ]
}

# On some BaseOS DHCP setups the lease gets an address and DNS but no default
# route (the router option is missing/ignored).  Without this route the local
# Web UI still works, while every WebDAV request fails with "Network is
# unreachable".  Use the DHCP gateway convention (first host in the subnet)
# when the default route is absent.
ensure_default_route() {
	wlan_default_route && return 0
	ip=$(wlan_ipv4 || true)
	[ -n "$ip" ] || return 1
	gw="${ip%.*}.1"
	"$IP" route add default via "$gw" dev "$WLAN" >/dev/null 2>&1 || true
	if wlan_default_route; then
		debug "up: added default route via $gw"
		return 0
	fi
	log "无法添加默认路由（网关 $gw）"
	return 1
}

# Some DHCP servers hand out no DNS option; the local UI still works but
# WebDAV hostnames cannot resolve.  Keep DHCP's values when present and only
# fall back to the gateway when resolv.conf is empty.
ensure_resolv() {
	resolv_ok && return 0
	gw=$(default_gateway || true)
	if [ -z "$gw" ]; then
		ip=$(wlan_ipv4 || true)
		[ -n "$ip" ] || return 1
		gw="${ip%.*}.1"
	fi
	[ -n "$gw" ] || return 1
	if printf 'nameserver %s\n' "$gw" >"$RESOLV_CONF" 2>/dev/null && resolv_ok; then
		debug "up: wrote fallback DNS nameserver $gw"
		return 0
	fi
	log "无法写入 DNS 配置 $RESOLV_CONF"
	return 1
}

supplicant_stop() {
	"$WPA_CLI" -p "$CTRL_DIR" -i "$WLAN" terminate >/dev/null 2>&1 || true
	i=0
	while [ -e "$CTRL_DIR/$WLAN" ] && [ "$i" -lt 8 ]; do
		sleep_short
		i=$((i + 1))
	done
	[ -e "$CTRL_DIR/$WLAN" ] && /bin/rm -f "$CTRL_DIR/$WLAN" 2>/dev/null
	/bin/rm -rf "$RUN/slot-wifi" 2>/dev/null

	# A scan that was cancelled can leave its temporary wpa_supplicant
	# running.  Two supplicants on the same interface make association fail
	# even though the new config is valid, so stop every daemon for this
	# interface before starting ours.
	killed=0
	for proc in /proc/[0-9]*; do
		[ -r "$proc/cmdline" ] || continue
		cmd=$(tr '\0' ' ' <"$proc/cmdline" 2>/dev/null) || continue
		case "$cmd" in
		*wpa_supplicant*"-i $WLAN"* | *wpa_supplicant*"-i$WLAN"*)
			pid=${proc##*/}
			kill "$pid" 2>/dev/null || true
			killed=$((killed + 1))
			;;
		esac
	done
	if [ "$killed" -gt 0 ]; then
		sleep_short
		sleep_short
		for proc in /proc/[0-9]*; do
			[ -r "$proc/cmdline" ] || continue
			cmd=$(tr '\0' ' ' <"$proc/cmdline" 2>/dev/null) || continue
			case "$cmd" in
			*wpa_supplicant*"-i $WLAN"* | *wpa_supplicant*"-i$WLAN"*)
				kill -9 "${proc##*/}" 2>/dev/null || true
				;;
			esac
		done
		debug "up: stopped $killed stale wpa_supplicant process(es) for $WLAN"
	fi
	return 0
}

# A previous connect can leave a DHCP client running.  Two clients on the
# same interface race for the lease and routes, so remove ours before starting
# a new one.
dhcp_stop() {
	killed=0
	for proc in /proc/[0-9]*; do
		[ -r "$proc/cmdline" ] || continue
		cmd=$(tr '\0' ' ' <"$proc/cmdline" 2>/dev/null) || continue
		case "$cmd" in
		*udhcpc*"-i $WLAN"* | *udhcpc*"-i$WLAN"*)
			pid=${proc##*/}
			kill "$pid" 2>/dev/null || true
			killed=$((killed + 1))
			;;
		esac
	done
	if [ "$killed" -gt 0 ]; then
		sleep_short
		for proc in /proc/[0-9]*; do
			[ -r "$proc/cmdline" ] || continue
			cmd=$(tr '\0' ' ' <"$proc/cmdline" 2>/dev/null) || continue
			case "$cmd" in
			*udhcpc*"-i $WLAN"* | *udhcpc*"-i$WLAN"*)
				kill -9 "${proc##*/}" 2>/dev/null || true
				;;
			esac
		done
		debug "up: stopped $killed stale udhcpc process(es) for $WLAN"
	fi
	return 0
}

# A WiFi driver that was left in a half-associated state sometimes refuses a
# new association until the interface has been cycled.  The caller only gets
# here when the previous state is not a usable connection.
wlan_reset() {
	"$IP" link set "$WLAN" down >/dev/null 2>&1 || true
	sleep_short
	"$IP" link set "$WLAN" up >/dev/null 2>&1 || true
	sleep_short
	return 0
}

# Escape backslash and double quote for a wpa_supplicant quoted string.
escape_wpa() {
	printf '%s' "$1" | sed 's/[\\"]/\\&/g'
}

write_wpa_conf() {
	mkdir -p "$CTRL_DIR" 2>/dev/null || true
	mkdir -p "$RUN" 2>/dev/null || true
	(
		umask 077
		{
			echo "ctrl_interface=$CTRL_DIR"
			echo 'update_config=0'
			echo 'network={'
			echo "	ssid=\"$(escape_wpa "$SSID")\""
			if [ -n "$PSK" ]; then
				echo '	key_mgmt=WPA-PSK'
				echo "	psk=\"$(escape_wpa "$PSK")\""
			else
				echo '	key_mgmt=NONE'
			fi
			echo '}'
		} >"$WPA_CONF"
	)
	chmod 600 "$WPA_CONF" 2>/dev/null || true
}

wait_association() {
	# ASSOC_WAIT is in seconds, but sleep_short is 0.25s; poll four times
	# per second so the association budget is not accidentally 1/4 of the
	# configured time.
	i=0
	limit=$((ASSOC_WAIT * 4))
	while :; do
		state=$(supplicant_state)
		[ "$state" = COMPLETED ] && return 0
		[ "$i" -ge "$limit" ] && return 1
		sleep_short
		i=$((i + 1))
	done
}

wait_ipv4() {
	i=0
	limit=$((DHCP_WAIT * 4))
	ip=$(wlan_ipv4 || true)
	while :; do
		[ -n "$ip" ] && return 0
		[ "$i" -ge "$limit" ] && return 1
		sleep_short
		i=$((i + 1))
		ip=$(wlan_ipv4 || true)
	done
}

wait_resolv() {
	i=0
	while [ ! -s "$RESOLV_CONF" ] && [ "$i" -lt 8 ]; do
		sleep_short
		i=$((i + 1))
	done
	[ -s "$RESOLV_CONF" ] || log "警告：未生成 $RESOLV_CONF，DNS 可能不可用"
	return 0
}

do_status() {
	debug "status: exists=$([ -d "$NET/$WLAN" ] && echo yes || echo no) operstate=$(cat "$NET/$WLAN/operstate" 2>/dev/null || echo unknown) wpa_state=$(supplicant_state || true) ssid=$(supplicant_ssid || true) ip=$(wlan_ipv4 || true)"
	if ! wlan_exists; then
		log "找不到无线网卡 $WLAN"
		return 1
	fi
	if ! wlan_link_up; then
		log "$WLAN 未启用"
		return 1
	fi
	if ! supplicant_available; then
		log "缺少 $WPA_CLI，无法确认关联状态"
		return 1
	fi
	state=$(supplicant_state)
	if [ "$state" != COMPLETED ]; then
		log "Wi-Fi 未关联（wpa_state=${state:-unknown}）"
		return 1
	fi
	ip=$(wlan_ipv4 || true)
	if [ -z "$ip" ]; then
		log "Wi-Fi 已关联但未获取 IPv4 地址"
		return 1
	fi
	if ! wlan_default_route; then
		log "Wi-Fi 已连接但缺少默认路由"
		return 1
	fi
	if ! resolv_ok; then
		log "Wi-Fi 已连接但缺少 DNS 配置"
		return 1
	fi
	log "已连接：ssid=$(supplicant_ssid) ip=$ip"
	return 0
}

do_up() {
	if do_status >/dev/null 2>&1; then
		log "网络已连接"
		return 0
	fi

	command -v "$WPA_SUPPLICANT" >/dev/null 2>&1 || {
		log "缺少 $WPA_SUPPLICANT，无法自动联网"
		return 1
	}
	supplicant_available || {
		log "缺少 $WPA_CLI，无法控制 Wi-Fi 关联"
		return 1
	}
	command -v "$UDHCPC" >/dev/null 2>&1 || {
		log "缺少 $UDHCPC，无法获取 IP 地址"
		return 1
	}
	if ! wlan_exists; then
		log "找不到无线网卡 $WLAN"
		return 1
	fi

	log "启用 Wi-Fi（$WLAN）"
	debug "up: rfkill=$(command -v "$RFKILL" && echo yes || echo no) ssid=$SSID timeout=$TIMEOUT assoc_wait=$ASSOC_WAIT dhcp_wait=$DHCP_WAIT"
	if command -v "$RFKILL" >/dev/null 2>&1; then
		"$RFKILL" unblock wifi >/dev/null 2>&1 || true
	fi
	"$IP" link set "$WLAN" up >/dev/null 2>&1 || true

	state=$(supplicant_state)
	cur_ssid=$(supplicant_ssid)
	reuse=0
	if [ "$state" = COMPLETED ] && [ -n "$SSID" ] && [ "$cur_ssid" = "$SSID" ]; then
		log "复用已关联的 Wi-Fi（ssid=$cur_ssid）"
		reuse=1
	fi

	if [ "$reuse" -eq 0 ]; then
		if [ -z "$SSID" ]; then
			log "wifi.ssid 未配置，无法自动联网"
			return 1
		fi
		log "停止旧的 wpa_supplicant（如有）"
		supplicant_stop
		dhcp_stop
		log "复位无线接口（$WLAN）"
		wlan_reset
		write_wpa_conf
		debug "up: wrote $WPA_CONF (ctrl_interface=$CTRL_DIR, psk_set=$([ -n "$PSK" ] && echo yes || echo no))"
		log "正在关联 Wi-Fi：$SSID"
		"$WPA_SUPPLICANT" -B -i "$WLAN" -c "$WPA_CONF" -Dnl80211 >/dev/null 2>&1 ||
			"$WPA_SUPPLICANT" -B -i "$WLAN" -c "$WPA_CONF" >/dev/null 2>&1 || {
			log "启动 wpa_supplicant 失败"
			return 1
		}
		if ! wait_association; then
			state=$(supplicant_state)
			oper=$(cat "$NET/$WLAN/operstate" 2>/dev/null || echo unknown)
			log "关联 Wi-Fi 超时（${ASSOC_WAIT}s，wpa_state=${state:-unknown}，operstate=${oper}）"
			return 1
		fi
		debug "up: associated ssid=$SSID"
		log "Wi-Fi 已关联：$SSID"
	fi

	ip=$(wlan_ipv4 || true)
	if [ -z "$ip" ]; then
		log "正在获取 IP 地址（udhcpc）"
		dhcp_stop
		"$UDHCPC" -i "$WLAN" -b >/dev/null 2>&1 || {
			log "udhcpc 启动失败"
			return 1
		}
		if ! wait_ipv4; then
			oper=$(cat "$NET/$WLAN/operstate" 2>/dev/null || echo unknown)
			log "获取 IPv4 地址超时（${DHCP_WAIT}s，operstate=${oper}）"
			return 1
		fi
		ip=$(wlan_ipv4 || true)
	fi
	if ! ensure_default_route; then
		return 1
	fi
	wait_resolv
	if ! ensure_resolv; then
		return 1
	fi
	log "网络已连接：ip=$ip"
	return 0
}

case "${1:-}" in
status) do_status ;;
up) do_up ;;
*)
	echo "usage: linkslot-net.sh status|up" >&2
	exit 2
	;;
esac
