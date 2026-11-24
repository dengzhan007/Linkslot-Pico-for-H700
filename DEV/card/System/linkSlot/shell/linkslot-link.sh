#!/bin/sh
set -u

NET="${AGS_NET_SYS:-/sys/class/net}"
RUN="${AGS_RUN:-/run}"

LINK_SSID="${AGS_LINK_SSID:-slotlink}"
LINK_PSK="${AGS_LINK_PSK:-slotlink0}"
LINK_FREQ="${AGS_LINK_FREQ:-5745}"
LINK_HOST_IP="${AGS_LINK_HOST_IP:-10.42.0.1/24}"
LINK_PEER_IP="${AGS_LINK_PEER_IP:-10.42.0.2/24}"
LINK_WAIT_S="${AGS_LINK_WAIT_S:-10}"
LINK_PIN_WAIT_S="${AGS_LINK_PIN_WAIT_S:-20}"
LINK_RESCAN="${AGS_LINK_RESCAN:-2}"
LINK_AP_GLOBAL="${AGS_LINK_AP_GLOBAL:-}"
LINK_TRACE="${AGS_LINK_TRACE:-}"
LINK_ROOT="${SLOT_ROOT:-/mnt/sdcard}"
LINK_PAUSE_HOME_WIFI="${AGS_LINK_PAUSE_HOME_WIFI:-1}"
LINK_RESTORE_WIFI="${AGS_LINK_RESTORE_WIFI:-1}"
LINK_MARK="$RUN/slotlink.session"
LINK_WIFI_PAUSED="$RUN/slotlink-wifi-paused"
LINK_WIFI_RESTORING="$RUN/slotlink-wifi-restoring"

WPA_SUPPLICANT="${AGS_WPA_SUPPLICANT:-wpa_supplicant}"
WPA_CLI="${AGS_WPA_CLI:-wpa_cli}"
IP="${AGS_IP:-ip}"
UPTIME="${AGS_UPTIME:-/proc/uptime}"
CTRL_DIR="${AGS_CTRL_DIR:-/var/run/wpa_supplicant}"

trace() {
	[ -n "$LINK_TRACE" ] || return 0
	printf '%s [debug] [link] slotlink %s\n' \
		"$(date '+%Y-%m-%dT%H:%M:%S' 2>/dev/null)" "$*" >> "$LINK_TRACE" 2>/dev/null
	return 0
}

addr_of() {
	[ -n "$LINK_TRACE" ] || return 0
	$IP addr show dev "$1" 2>/dev/null | awk '/inet / { print $2; exit }'
}

wpa_state() {
	[ -n "$LINK_TRACE" ] || return 0
	$WPA_CLI -p "$1" -i "$2" status 2>/dev/null | sed -n 's/^wpa_state=//p'
}

wpa_state_raw() {
	$WPA_CLI -p "$1" -i "$2" status 2>/dev/null | sed -n 's/^wpa_state=//p'
}

wpa_freq() {
	[ -n "$LINK_TRACE" ] || return 0
	$WPA_CLI -p "$1" -i "$2" status 2>/dev/null | sed -n 's/^freq=//p'
}

pause_home_wifi() {
	if [ "$LINK_PAUSE_HOME_WIFI" = 0 ]; then
		trace "pause: disabled by AGS_LINK_PAUSE_HOME_WIFI=0"
		return 0
	fi
	# A single-radio H700 shares wlan0 and wlan1: leaving the home STA
	# associated while hosting the link AP makes the cable stream miss its
	# 60-frame quiet window.  Take wlan0 down for the duration of the link;
	# restore_home_wifi() asks linkSlot to reconnect the saved WiFi afterwards.
	wait=0
	while [ -f "$LINK_WIFI_RESTORING" ] && [ "$wait" -lt 40 ]; do
		sleep 0.25 2>/dev/null || sleep 1
		wait=$((wait + 1))
	done
	state=$(wpa_state_raw "$CTRL_DIR" wlan0)
	case "$state" in
	COMPLETED | ASSOCIATED | 4WAY_HANDSHAKE | GROUP_HANDSHAKE | AUTHENTICATING)
		: > "$LINK_WIFI_PAUSED"
		trace "pause: home WiFi state=$state; giving wlan0 to the link"
		supplicant_stop "$CTRL_DIR" wlan0
		$IP addr flush dev wlan0 2>/dev/null || true
		$IP link set wlan0 down 2>/dev/null || true
		trace "pause: wlan0 down"
		;;
	*)
		trace "pause: no home WiFi state=${state:-none}; leaving wlan0 alone"
		;;
	esac
}

restore_home_wifi() {
	[ -f "$LINK_WIFI_PAUSED" ] || return 0
	/bin/rm -f "$LINK_WIFI_PAUSED"
	if [ "$LINK_RESTORE_WIFI" = 0 ]; then
		trace "restore: disabled by AGS_LINK_RESTORE_WIFI=0"
		return 0
	fi
	if [ ! -f "$LINK_ROOT/System/linkSlot/shell/linkslot.sh" ]; then
		trace "restore: no linkslot.sh; reconnect WiFi from 网络设置"
		return 0
	fi
	if [ -f "$LINK_WIFI_RESTORING" ]; then
		trace "restore: already running"
		return 0
	fi
	: > "$LINK_WIFI_RESTORING"
	trace "restore: starting saved WiFi in the background"
	(
		SLOT_ROOT="$LINK_ROOT" /bin/sh "$LINK_ROOT/System/linkSlot/shell/linkslot.sh" net up \
			>/dev/null 2>&1
		/bin/rm -f "$LINK_WIFI_RESTORING"
	) &
}

wait_dev() {
	w=0
	while [ ! -d "$NET/$1" ] && [ "$w" -lt "$2" ]; do
		sleep 0.25 2>/dev/null || sleep 1
		w=$((w + 1))
	done
	[ -d "$NET/$1" ]
}

wifi_up() {
	wait_dev wlan0 40 || return 0
	trace "wlan0 present"
	rfkill unblock wifi 2>/dev/null || true
	$IP link set wlan0 up 2>/dev/null || true
	trace "wlan0 up addr=$(addr_of wlan0) state=$(wpa_state "$CTRL_DIR" wlan0)"
}

supplicant_stop() {
	if $WPA_CLI -p "$1" -i "$2" terminate >/dev/null 2>&1; then
		s=0
		while [ -e "$1/$2" ] && [ "$s" -lt 8 ]; do
			sleep 0.25 2>/dev/null || sleep 1
			s=$((s + 1))
		done
		trace "$2: supplicant terminated"
	elif [ -e "$1/$2" ]; then
		trace "$2: stale socket removed"
		/bin/rm -f "$1/$2"
	fi
	return 0
}

link_clear() {
	trace "clear: begin wlan0=$(addr_of wlan0) wlan1=$(addr_of wlan1)"
	supplicant_stop "$RUN/wpa_ap" wlan1
	$IP addr flush dev wlan1 2>/dev/null || true
	/bin/rm -f "$RUN/slotlink-ap.conf"
	if [ -f "$RUN/slotlink-sta.conf" ]; then
		trace "clear: stale join config; taking wlan0 away from home WiFi"
		supplicant_stop "$CTRL_DIR" wlan0
		$IP addr flush dev wlan0 2>/dev/null || true
		/bin/rm -f "$RUN/slotlink-sta.conf"
	else
		trace "clear: no stale join config; leaving wlan0 alone"
	fi
	trace "clear: end"
}

link_wait() {
	p=0
	was=
	end=$(($(date +%s) + $3))
	while :; do
		state=$($WPA_CLI -p "$1" -i "$2" status 2>/dev/null | sed -n 's/^wpa_state=//p')
		if [ "$state" != "$was" ]; then
			trace "$2 ${state:-no answer} freq=$(wpa_freq "$1" "$2")"
			was=$state
		fi
		[ "$state" = COMPLETED ] && return 0
		[ "$(date +%s)" -ge "$end" ] && return 1
		if [ "$4" -gt 0 ] && [ "$p" -gt 0 ] && [ $((p % $4)) -eq 0 ]; then
			case "$state" in
			DISCONNECTED | INACTIVE | SCANNING)
				$WPA_CLI -p "$1" -i "$2" scan ${5:+freq=$5} >/dev/null 2>&1
				;;
			esac
		fi
		sleep 0.25 2>/dev/null || sleep 1
		p=$((p + 1))
	done
}

link_host() {
	echo "$$" > "$LINK_MARK"
	trace "host: asked"
	pause_home_wifi
	link_clear
	wifi_up
	if ! wait_dev wlan1 4; then
		trace "host: wlan1 missing after 1s"
		return 1
	fi
	trace "host: wlan1 present addr=$(addr_of wlan1)"
	if ! command -v "$WPA_SUPPLICANT" >/dev/null 2>&1; then
		trace "host: wpa_supplicant missing"
		return 1
	fi

	/bin/mkdir -p "$RUN/wpa_ap"
	{
		echo "ctrl_interface=$RUN/wpa_ap"
		echo 'ap_scan=2'
		if [ -n "$LINK_AP_GLOBAL" ]; then
			printf '%s\n' "$LINK_AP_GLOBAL"
		fi
		echo 'network={'
		echo "	ssid=\"$LINK_SSID\""
		echo '	mode=2'
		echo "	frequency=$LINK_FREQ"
		echo '	key_mgmt=WPA-PSK'
		echo '	proto=RSN'
		echo '	pairwise=CCMP'
		echo "	psk=\"$LINK_PSK\""
		echo '}'
	} > "$RUN/slotlink-ap.conf"
	/bin/chmod 600 "$RUN/slotlink-ap.conf"

	$IP link set wlan1 up 2>/dev/null || true
	$WPA_SUPPLICANT -B -i wlan1 -c "$RUN/slotlink-ap.conf" -Dnl80211 >/dev/null 2>&1
	sup=$?
	trace "host: supplicant exit=$sup"
	[ "$sup" -eq 0 ] || return 1
	link_wait "$RUN/wpa_ap" wlan1 "$LINK_WAIT_S" 0 "" || {
		trace "host: AP did not reach COMPLETED; state=$(wpa_state "$RUN/wpa_ap" wlan1)"
		return 1
	}
	$IP addr add "$LINK_HOST_IP" dev wlan1 2>/dev/null
	addr=$?
	trace "host: addr add 10.42.0.1 exit=$addr addr=$(addr_of wlan1) freq=$(wpa_freq "$RUN/wpa_ap" wlan1)"
	trace "host: ready"
}

link_sta_conf() {
	{
		echo "ctrl_interface=$CTRL_DIR"
		echo 'network={'
		echo "	ssid=\"$LINK_SSID\""
		echo '	key_mgmt=WPA-PSK'
		echo "	psk=\"$LINK_PSK\""
		if [ -n "${1:-}" ]; then
			echo "	scan_freq=$1"
			echo "	freq_list=$1"
		fi
		echo '}'
	} > "$RUN/slotlink-sta.conf"
	/bin/chmod 600 "$RUN/slotlink-sta.conf"
}

link_join() {
	echo "$$" > "$LINK_MARK"
	trace "join: asked"
	pause_home_wifi
	link_clear
	wifi_up
	if [ ! -d "$NET/wlan0" ]; then
		trace "join: wlan0 missing"
		return 1
	fi
	/bin/mkdir -p "$CTRL_DIR"
	$IP link set wlan0 up 2>/dev/null || true

	tries="${AGS_LINK_TRIES:-2}"
	attempt=1
	while [ "$attempt" -le "$tries" ]; do
		# The first pass scans every channel: hard-pinning 5745 can spend the
		# whole 20 s budget scanning a band the AP is not actually using.  The
		# retry then pins the configured host channel for a stubborn driver.
		if [ "$attempt" -eq 1 ]; then
			freq=
			bound="$LINK_PIN_WAIT_S"
		else
			freq="$LINK_FREQ"
			bound="$LINK_WAIT_S"
		fi
		link_sta_conf "$freq"
		supplicant_stop "$CTRL_DIR" wlan0
		$WPA_SUPPLICANT -B -i wlan0 -c "$RUN/slotlink-sta.conf" -Dnl80211 >/dev/null 2>&1
		sup=$?
		trace "join: attempt=$attempt freq=${freq:-any} supplicant_exit=$sup bound=${bound}s"
		if [ "$sup" -ne 0 ]; then
			attempt=$((attempt + 1))
			continue
		fi
		if link_wait "$CTRL_DIR" wlan0 "$bound" "$LINK_RESCAN" "$freq"; then
			$IP addr add "$LINK_PEER_IP" dev wlan0 2>/dev/null
			addr=$?
			trace "join: addr add 10.42.0.2 exit=$addr addr=$(addr_of wlan0) freq=$(wpa_freq "$CTRL_DIR" wlan0)"
			trace "join: ready"
			return 0
		fi
		trace "join: attempt=$attempt failed state=$(wpa_state "$CTRL_DIR" wlan0) freq=$(wpa_freq "$CTRL_DIR" wlan0)"
		attempt=$((attempt + 1))
	done
	trace "join: no host found after $tries attempts"
	return 3
}

link_down() {
	trace "down: begin"
	link_clear
	$IP link set wlan1 down 2>/dev/null || true
	rmdir "$RUN/wpa_ap" 2>/dev/null || true
	/bin/rm -f "$LINK_MARK"
	restore_home_wifi
	trace "down: end"
	return 0
}

link_warm() {
	trace "warm: begin"
	wifi_up
	[ -d "$NET/wlan0" ] || {
		trace "warm: wlan0 missing"
		return 1
	}
	wait_dev wlan1 4
	trace "warm: end wlan0=$(addr_of wlan0) wlan1=$(addr_of wlan1)"
}

case "${1:-} ${2:-}" in
"link host") link_host ;;
"link join") link_join ;;
"link down") link_down ;;
"link warm") link_warm ;;
"link cool") exit 0 ;;
*)
	echo "usage: slotlink.sh link [host|join|down|warm|cool]" >&2
	exit 2
	;;
esac
