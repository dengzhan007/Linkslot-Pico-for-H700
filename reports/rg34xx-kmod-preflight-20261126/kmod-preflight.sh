K=$(uname -r)
echo "=== kernel ==="
uname -a
echo "=== module config ==="
zcat /proc/config.gz 2>/dev/null | grep -E "^(CONFIG_MODULES=|CONFIG_MODULE_UNLOAD=|CONFIG_KALLSYMS=|CONFIG_DEVMEM=|# CONFIG_STRICT_DEVMEM)" || true
echo "=== kernel build files ==="
for P in \
/lib/modules/$K \
/lib/modules/$K/build \
/lib/modules/$K/build/Makefile \
/lib/modules/$K/build/Module.symvers \
/usr/src/linux \
/usr/src/linux-$K \
/usr/src/linux-headers-$K
do
if [ -e "$P" ]; then ls -ld "$P"; else echo "MISSING $P"; fi
done
echo "=== relevant kernel symbols (names only) ==="
grep -E " [Tt] (ioremap|iounmap|__ioremap|readl|regmap_init_mmio|devm_ioremap_resource)$" /proc/kallsyms 2>/dev/null | awk "{print \$3}" | sort -u
echo "=== existing platform drivers ==="
ls -1 /sys/bus/platform/drivers 2>/dev/null | grep -Ei "sys|sunxi|ccu|regmap" || true
echo "=== existing debugfs regmap ==="
ls -1 /sys/kernel/debug/regmap 2>/dev/null || true
echo "=== relevant iomem entries ==="
grep -Ei "03000000|syscfg|syscon" /proc/iomem || true
