LOCAL_DIR := $(GET_LOCAL_DIR)

ARCH := arm
ARM_CPU := cortex-a53
CPU := generic

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
	$(LOCAL_DIR)/power/h700_cpu_performance_runtime.o

# No H700 MMIO drivers are enabled here yet. Add modules only after their
# register, clock, pinmux and ownership assumptions have separate evidence.
