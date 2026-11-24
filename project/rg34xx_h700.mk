# RG34XX (non-SP) / Allwinner H700 / Cortex-A53
LOCAL_DIR := $(GET_LOCAL_DIR)

H700_BUILD := yes
TARGET := rg34xx_h700
TOOLCHAIN_PREFIX := arm-none-eabi-
ENABLE_THUMB := true

# This target has no MediaTek DWS/drvgen inputs.
USE_DTB_NO_DWS := yes

DEFINES += H700_BUILD
