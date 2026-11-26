LOCAL_DIR := $(GET_LOCAL_DIR)

ARCH := arm
ARM_CPU := cortex-a53
CPU := generic

# Verified RG34XX physical DRAM base used by GammaOS AArch32 arm_mmu_init().
DEFINES += DRAM_PHY_ADDR=0x40000000

# LK build.mk links the ELF with this generated one-segment script.
# The target's fail-closed memory gate must accept explicit reviewed values
# before the linker script can be generated with device addresses.
LINKER_SCRIPT += $(BUILDDIR)/system-onesegment.ld

# Policy-only CPU performance service and its static owner.
# These objects have no MMIO, PMIC, PLL or thermal-sensor access. They are
# linked now so the real LK target can resolve the C ABI when a future,
# separately validated board startup path is added.
OBJS += \
	$(LOCAL_DIR)/power/h700_cpu_performance_policy.o \
	$(LOCAL_DIR)/power/h700_cpu_performance_service.o \
	$(LOCAL_DIR)/power/h700_cpu_performance_runtime.o \
	$(LOCAL_DIR)/power/h700_rg34xx_power_led.o \
	$(LOCAL_DIR)/runtime/lk_heap_service.o \
	$(LOCAL_DIR)/display/h700_display_refresh_policy.o \
	$(LOCAL_DIR)/arch/h700_pio_mmu_mapping.o

# RG34XX PI12 power LED is the first board MMIO service. It only changes the
# GPIO mux/data fields; it does not reprogram CCU/APB1 or touch the PMIC.
# Other H700 MMIO drivers remain disabled pending their own evidence.
