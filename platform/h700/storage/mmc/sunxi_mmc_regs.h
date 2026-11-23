#ifndef H700_SUNXI_MMC_REGS_H
#define H700_SUNXI_MMC_REGS_H

#include <stdint.h>

/* Synopsys/Allwinner SD/MMC register block offsets (H616/H700 variant). */
#define SUNXI_MMC_REG_GCTRL       0x000u
#define SUNXI_MMC_REG_CLKCR       0x004u
#define SUNXI_MMC_REG_TMOUT       0x008u
#define SUNXI_MMC_REG_WIDTH       0x00cu
#define SUNXI_MMC_REG_BLKSZ       0x010u
#define SUNXI_MMC_REG_BYTECNT     0x014u
#define SUNXI_MMC_REG_CMD         0x018u
#define SUNXI_MMC_REG_ARG         0x01cu
#define SUNXI_MMC_REG_RESP0       0x020u
#define SUNXI_MMC_REG_RESP1       0x024u
#define SUNXI_MMC_REG_RESP2       0x028u
#define SUNXI_MMC_REG_RESP3       0x02cu
#define SUNXI_MMC_REG_IMASK       0x030u
#define SUNXI_MMC_REG_MISTA       0x034u
#define SUNXI_MMC_REG_RINT        0x038u
#define SUNXI_MMC_REG_STATUS      0x03cu
#define SUNXI_MMC_REG_FTRGL       0x040u
#define SUNXI_MMC_REG_FUNS        0x044u
#define SUNXI_MMC_REG_CBCR        0x048u
#define SUNXI_MMC_REG_BBCR        0x04cu
#define SUNXI_MMC_REG_DBGC        0x050u
#define SUNXI_MMC_REG_A12A        0x058u
#define SUNXI_MMC_REG_NTSR        0x05cu
#define SUNXI_MMC_REG_HWRST       0x078u
#define SUNXI_MMC_REG_DMAC        0x080u
#define SUNXI_MMC_REG_DLBA        0x084u
#define SUNXI_MMC_REG_IDST        0x088u
#define SUNXI_MMC_REG_IDIE        0x08cu
#define SUNXI_MMC_REG_CHDA        0x090u
#define SUNXI_MMC_REG_CBDA        0x094u
#define SUNXI_MMC_REG_THLDC       0x100u
#define SUNXI_MMC_REG_SAMP_DL     0x144u
#define SUNXI_MMC_REG_FIFO        0x200u

/* CLKCR */
#define SUNXI_MMC_CLKCR_DIVIDER_MASK 0x000000ffu
#define SUNXI_MMC_CLKCR_CARD_CLOCK  (1u << 16)
#define SUNXI_MMC_CLKCR_LOW_POWER   (1u << 17)
/* H616: mask DAT0 while issuing UPCLK_ONLY clock-update commands. */
#define SUNXI_MMC_CLKCR_MASK_DATA0  (1u << 31)

/* CMD */
#define SUNXI_MMC_CMD_WAIT_PRE_OVER (1u << 13)
#define SUNXI_MMC_CMD_UPCLK_ONLY    (1u << 21)
#define SUNXI_MMC_CMD_START         (1u << 31)

/* RINT */
#define SUNXI_MMC_RINT_SDIO         (1u << 16)

/* GCTRL */
#define SUNXI_MMC_GCTRL_SOFT_RESET  (1u << 0)
#define SUNXI_MMC_GCTRL_FIFO_RESET  (1u << 1)
#define SUNXI_MMC_GCTRL_DMA_RESET   (1u << 2)
#define SUNXI_MMC_GCTRL_INT_ENABLE  (1u << 4)
#define SUNXI_MMC_GCTRL_DMA_ENABLE  (1u << 5)
#define SUNXI_MMC_GCTRL_ACCESS_AHB  (1u << 31)
#define SUNXI_MMC_GCTRL_RESET_MASK  (SUNXI_MMC_GCTRL_SOFT_RESET | \
                                     SUNXI_MMC_GCTRL_FIFO_RESET | \
                                     SUNXI_MMC_GCTRL_DMA_RESET)

/* H616 CCU offsets. These are offsets from the verified CCU base, not MMIO bases. */
#define SUNXI_CCU_MMC0_CLK_CFG     0x830u
#define SUNXI_CCU_MMC1_CLK_CFG     0x834u
#define SUNXI_CCU_MMC2_CLK_CFG     0x838u
#define SUNXI_CCU_MMC_GATE_RESET   0x84cu
#define SUNXI_CCU_MMC_GATE_BIT(n)  (1u << (n))
#define SUNXI_CCU_MMC_RESET_BIT(n) (1u << (16u + (n)))

#endif /* H700_SUNXI_MMC_REGS_H */
