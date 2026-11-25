#!/bin/sh
LABEL="${1:-unknown}"
case "$LABEL" in ui|game) ;; *) echo "usage: $0 ui|game"; exit 2 ;; esac
OUT="/tmp/slot-refresh-$LABEL.txt"
exec > "$OUT" 2>&1

echo "=== SLOT REFRESH AUDIT: $LABEL ==="
date
id
uname -a

echo
echo "=== BASEOS ==="
cat /etc/baseos-release 2>&1

echo
echo "=== PANEL ==="
DT=/sys/firmware/devicetree/base
ALIAS=$(tr -d '\000' < "$DT/aliases/lcd0" 2>/dev/null)
PANEL="$DT$ALIAS"
echo "alias=$ALIAS"
echo "node=$PANEL"

if [ ! -d "$PANEL" ]; then
  PANEL=$(find "$DT" -name lcd_driver_name -type f 2>/dev/null | head -n 1 | sed 's#/lcd_driver_name$##')
fi

for n in lcd_driver_name lcd_x lcd_y lcd_ht lcd_vt lcd_dclk_freq lcd_if; do
  f="$PANEL/$n"
  if [ ! -r "$f" ]; then
    echo "$n=MISSING"
  elif [ "$n" = lcd_driver_name ]; then
    echo "$n=$(tr -d '\000' < "$f")"
  else
    h=$(od -An -v -tx1 "$f" 2>/dev/null | tr -d ' \n')
    if [ "${#h}" -eq 8 ]; then
      echo "$n=$((0x$h)) (BE 0x$h)"
    else
      echo "$n=raw:$h"
    fi
  fi
done

echo
echo "=== SLOT PROCESSES ==="
FOUND=0
for d in /proc/[0-9]*; do
  [ -r "$d/cmdline" ] || continue
  p=${d##*/}
  c=$(cat "$d/comm" 2>/dev/null)
  cmd=$(tr '\000' ' ' < "$d/cmdline" 2>/dev/null)
  exe=$(readlink "$d/exe" 2>/dev/null)
  maps=$(grep -E 'slot|mgba_libretro|gpsp_libretro|armgba' "$d/maps" 2>/dev/null | head -n 8)
  case "$c $cmd $exe $maps" in
    *slot*|*mgba_libretro*|*gpsp_libretro*|*armgba*)
      FOUND=1
      echo "--- PID=$p ---"
      echo "comm=$c"
      echo "cmdline=$cmd"
      echo "exe=$exe"
      echo "$maps"
      echo "environment:"
      tr '\000' '\n' < "$d/environ" 2>/dev/null | grep -E '^(BASEOS_TARGET|BASEOS_DEVICE|SLOT_ROOT)='
      root=$(tr '\000' '\n' < "$d/environ" 2>/dev/null | sed -n 's/^SLOT_ROOT=//p' | head -n 1)
      if [ -n "$root" ] && [ -r "$root/Config/slot.state" ]; then
        echo "slot.state:"
        grep '^high_refresh=' "$root/Config/slot.state"
      fi
      ;;
  esac
done
[ "$FOUND" = 1 ] || echo "No Slot process found"

echo
echo "=== DISPLAY ==="
cat /sys/class/disp/disp/attr/sys 2>&1

echo
echo "=== LCD IRQ 293 ==="
irq() {
  awk '$1=="293:" {s=0; for(i=2;i<=NF;i++){if($i~/^[0-9]+$/)s+=$i;else break} print s; found=1; exit} END{if(!found)print "NA"}' /proc/interrupts
}
B=$(irq)
echo "before=$B"
sleep 12
A=$(irq)
echo "after=$A"
if [ "$A" != NA ] && [ "$B" != NA ]; then
  awk -v b="$B" -v a="$A" 'BEGIN{printf "delta=%d\nmeasured_hz=%.3f\n",a-b,(a-b)/12}'
fi
grep -Ei 'lcd|disp|tcon|dsi' /proc/interrupts 2>/dev/null

echo
echo "=== REGISTER READBACK ONLY ==="
D=/sys/class/sunxi_dump/dump
if [ -w "$D" ]; then
  for addr in 0x06511044 0x03001040; do
    echo "request=$addr"
    printf '%s,%s\n' "$addr" "$addr" > "$D" 2>&1
    cat "$D" 2>&1
  done
else
  echo "dump interface unavailable or not writable"
fi

echo
echo "WRITE INTERFACE WAS NEVER ACCESSED"
echo "REPORT=$OUT"
