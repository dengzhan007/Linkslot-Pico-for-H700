LOCAL_DIR := $(GET_LOCAL_DIR)
PLATFORM := h700

# This is an intentional hard stop, not a default memory map. A separate
# board-level review must approve the exact range before a real LK build.
ifneq ($(strip $(H700_MEMORY_LAYOUT_APPROVED)),yes)
$(error H700_MEMORY_LAYOUT_APPROVED must be set to yes after the RG34XX memory layout is approved)
endif

ifeq ($(strip $(H700_MEMBASE)),)
$(error H700_MEMBASE is required after H700 memory-layout approval)
endif
ifeq ($(strip $(H700_MEMSIZE)),)
$(error H700_MEMSIZE is required after H700 memory-layout approval)
endif

H700_MEMBASE_HEX_VALID := $(shell printf '%s\n' '$(strip $(H700_MEMBASE))' | grep -Eq '^0[xX][0-9a-fA-F]+$$' && echo yes)
ifneq ($(H700_MEMBASE_HEX_VALID),yes)
$(error H700_MEMBASE must be a hexadecimal 0x... address)
endif

H700_MEMSIZE_HEX_VALID := $(shell printf '%s\n' '$(strip $(H700_MEMSIZE))' | grep -Eq '^0[xX][0-9a-fA-F]+$$' && echo yes)
ifneq ($(H700_MEMSIZE_HEX_VALID),yes)
$(error H700_MEMSIZE must be a hexadecimal 0x... byte count)
endif

H700_MEMBASE_NONZERO := $(shell printf '%s' '$(strip $(H700_MEMBASE))' | sed -E 's/^0[xX]//; s/^0+//' | grep -q '[1-9a-fA-F]' && echo yes)
ifneq ($(H700_MEMBASE_NONZERO),yes)
$(error H700_MEMBASE must be non-zero)
endif

H700_MEMSIZE_NONZERO := $(shell digits=$$(printf '%s' '$(strip $(H700_MEMSIZE))' | sed -E 's/^0[xX]//; s/^0+//'); test -n "$$digits" && echo yes)
ifneq ($(H700_MEMSIZE_NONZERO),yes)
$(error H700_MEMSIZE must be non-zero)
endif

# These are supplied by the operator only after a separate memory audit.
# Never replace the fail-closed defaults with guessed board addresses.
MEMBASE := $(H700_MEMBASE)
MEMSIZE := $(H700_MEMSIZE)
DEFINES += H700_MEMORY_LAYOUT_APPROVED
DEFINES += MEMBASE=$(MEMBASE) MEMSIZE=$(MEMSIZE)
