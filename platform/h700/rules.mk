LOCAL_DIR := $(GET_LOCAL_DIR)

ARCH := arm
ARM_CPU := cortex-a53
CPU := generic

# LK build.mk links the ELF with this generated one-segment script.
# The target's fail-closed memory gate must accept explicit reviewed values
# before the linker script can be generated with device addresses.
LINKER_SCRIPT += $(BUILDDIR)/system-onesegment.ld

# No H700 MMIO drivers are enabled here yet. Add modules only after their
# register, clock, pinmux and ownership assumptions have separate evidence.
