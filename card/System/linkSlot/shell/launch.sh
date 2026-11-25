#!/bin/sh
SD="${SLOT_ROOT:-/mnt/sdcard}"
SYS="$SD/System/linkSlot"
RUN="${AGS_RUN:-/run}"
LOGS="$SD/linkSlot/Logs"
SESSION_MAX=65536
export SLOT_ROOT="$SD"
read -r BOOT_START_UP _ < /proc/uptime 2>/dev/null || BOOT_START_UP=0

mkdir -p "$LOGS" 2>/dev/null
for f in slot.log slot.log.1 slot-session.log slot-session.log.1 slot-card.log slot-startup-r2.log slot-library.log ags-shutdown-trace.log input-trace.log; do
	[ -f "$SD/$f" ] && mv -f "$SD/$f" "$LOGS/$f"
done
DIAG="$LOGS/slot-startup-r2.log"
: > "$DIAG"
boot_trace() {
    read -r up _ < /proc/uptime 2>/dev/null || up=0
    printf '%s uptime=%s launcher: %s\n' "$(date '+%F %T')" "$up" "$*" >> "$DIAG"
}
boot_trace "start (captured uptime=$BOOT_START_UP)"
boot_trace "log move done"


[ $(wc -c < "$LOGS/slot-session.log" 2>/dev/null || echo 0) -gt "$SESSION_MAX" ] \
	&& mv -f "$LOGS/slot-session.log" "$LOGS/slot-session.log.1"
boot_trace "log rotation done"

log() {
	echo "$(date '+%F %T') $*" >> "$LOGS/slot-session.log" 2>/dev/null
}

pid="$(cat "$RUN/slot-led.pid" 2>/dev/null)"
if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
	boot_trace "led start"
	sh "$SYS/shell/led.sh" once
	sh "$SYS/shell/led.sh" loop > /dev/null 2>&1 &
	echo $! > "$RUN/slot-led.pid"
	boot_trace "led ready"
fi

pid="$(cat "$RUN/slot-lid.pid" 2>/dev/null)"
if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
	boot_trace "lid start"
	sh "$SYS/shell/lid.sh" loop > /dev/null 2>&1 &
	echo $! > "$RUN/slot-lid.pid"
	boot_trace "lid ready"
fi

echo "|/bin/sh $SYS/shell/coresave.sh %p %s %e %t" > /proc/sys/kernel/core_pattern 2>/dev/null

if [ -f "$RUN/slotlink.session" ]; then
	boot_trace "slotlink down start"
	sh "$SYS/shell/linkslot-link.sh" link down > /dev/null 2>&1
	boot_trace "slotlink down done"
fi

[ -f "$LOGS/slot.log" ] && mv -f "$LOGS/slot.log" "$LOGS/slot.log.1"
log "exec $SYS/bin/linkslot"
boot_trace "log files prepared"

# r2: keep a shell-side record even if the new executable never reaches main.
MARKER="$RUN/slot-zh-ready.$$"
export SLOT_BOOT_MARKER="$MARKER"
{
    echo "slot zh-lcd4-r2 launcher"
    date
    echo "root=$SD"
    uname -a
    ls -l "$SYS/bin/linkslot" /lib/ld-linux-aarch64.so.1
    free
} >> "$DIAG" 2>&1
echo "r2 launcher: starting modified Slot; details in Logs/slot-startup-r2.log" > "$LOGS/slot.log"
boot_trace "sync before app start"
sync
boot_trace "sync done"
/lib/ld-linux-aarch64.so.1 "$SYS/bin/linkslot" >> "$LOGS/slot.log" 2>&1 &
child=$!
boot_trace "app launched pid=$child"
echo "modified pid=$child" >> "$DIAG"
count=0
while [ "$count" -lt 150 ]; do
    [ -f "$MARKER" ] && break
    kill -0 "$child" 2>/dev/null || break
    # Poll fast when the shell supports fractional sleeps; the old one-second
    # step wasted up to a second of the visible boot.
    sleep 0.2 2>/dev/null || sleep 1
    count=$((count + 1))
done
if [ -f "$MARKER" ]; then
    boot_trace "first frame marker seen (poll=${count} polls)"
    echo "modified first frame confirmed after ${count}s" >> "$DIAG"
    wait "$child"
    status=$?
    rm -f "$MARKER"
    echo "modified exit=$status (after successful boot)" >> "$DIAG"
    exit "$status"
fi
boot_trace "first frame marker timeout"
echo "modified failed to show first frame; exit for init respawn" >> "$DIAG"
if kill -0 "$child" 2>/dev/null; then
    kill "$child" 2>/dev/null
    sleep 1
    kill -0 "$child" 2>/dev/null && kill -9 "$child" 2>/dev/null
fi
wait "$child"
status=$?
boot_trace "modified exit=$status (before first frame)"
echo "modified exit=$status (before first frame)" >> "$DIAG"
rm -f "$MARKER"
unset SLOT_BOOT_MARKER
exit 1
