LOCAL_DIR := $(GET_LOCAL_DIR)

ARCH := arm
ARM_CPU := cortex-a53
CPU := generic

# No H700 MMIO drivers are enabled here yet. Add modules only after their
# register, clock, pinmux and ownership assumptions have separate evidence.
