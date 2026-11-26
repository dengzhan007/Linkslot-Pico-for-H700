#include "h700_rg34xx_power_led.h"
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#define REG_WORDS 128u
#define MAX_WRITES 16u
struct fake { uint32_t regs[REG_WORDS], offsets[MAX_WRITES], writes, drop; };
static uint32_t rd(void *ctx, uint32_t off) { struct fake *f=ctx; assert(!(off&3u)); assert(off/4u<REG_WORDS); return f->regs[off/4u]; }
static void wr(void *ctx, uint32_t off, uint32_t val) { struct fake *f=ctx; assert(!(off&3u)); assert(off/4u<REG_WORDS); assert(f->writes<MAX_WRITES); f->offsets[f->writes++]=off; if (!f->drop) f->regs[off/4u]=val; }
static h700_rg34xx_power_led_io_v1_t io_for(struct fake *f) { h700_rg34xx_power_led_io_v1_t io={H700_RG34XX_POWER_LED_ABI_VERSION,sizeof(io),f,rd,wr}; return io; }
static uint32_t mux(const struct fake *f) { return f->regs[H700_RG34XX_POWER_LED_PIO_MUX1_OFFSET/4u]; }
static uint32_t data(const struct fake *f) { return f->regs[H700_RG34XX_POWER_LED_PIO_DATA_OFFSET/4u]; }

static void test_preload_and_bit_preservation(void) {
    struct fake f={0}; h700_rg34xx_power_led_service_v1_t svc={0}; h700_rg34xx_power_led_io_v1_t io=io_for(&f);
    uint32_t other_mux=0xA5C3D2E7u & ~H700_RG34XX_POWER_LED_PINMUX_MASK;
    f.regs[H700_RG34XX_POWER_LED_PIO_MUX1_OFFSET/4u]=other_mux|(7u<<16);
    f.regs[H700_RG34XX_POWER_LED_PIO_DATA_OFFSET/4u]=(1u<<11)|(1u<<5);
    assert(h700_rg34xx_power_led_init_v1(&svc,sizeof(svc),&io)==0);
    assert(svc.initialized && svc.game_active==0);
    assert(data(&f)&H700_RG34XX_POWER_LED_PIN_MASK);
    assert(data(&f)&(1u<<11)); assert(data(&f)&(1u<<5));
    assert((mux(&f)&H700_RG34XX_POWER_LED_PINMUX_MASK)==H700_RG34XX_POWER_LED_PINMUX_GPIO_OUT);
    assert((mux(&f)&~H700_RG34XX_POWER_LED_PINMUX_MASK)==other_mux);
    assert(f.writes==2 && f.offsets[0]==H700_RG34XX_POWER_LED_PIO_DATA_OFFSET && f.offsets[1]==H700_RG34XX_POWER_LED_PIO_MUX1_OFFSET);
}
static void test_game_lifecycle(void) {
    struct fake f={0}; h700_rg34xx_power_led_service_v1_t svc={0}; h700_rg34xx_power_led_io_v1_t io=io_for(&f);
    f.regs[H700_RG34XX_POWER_LED_PIO_MUX1_OFFSET/4u]=H700_RG34XX_POWER_LED_PINMUX_GPIO_OUT|(5u<<20);
    f.regs[H700_RG34XX_POWER_LED_PIO_DATA_OFFSET/4u]=1u<<11;
    assert(h700_rg34xx_power_led_init_v1(&svc,sizeof(svc),&io)==0);
    uint32_t n=f.writes;
    assert(h700_rg34xx_power_led_set_game_active_v1(&svc,1)==0);
    assert(!(data(&f)&H700_RG34XX_POWER_LED_PIN_MASK) && (data(&f)&(1u<<11)));
    assert(f.writes==n+1);
    n=f.writes; assert(h700_rg34xx_power_led_set_game_active_v1(&svc,1)==0 && f.writes==n);
    assert(h700_rg34xx_power_led_set_game_active_v1(&svc,0)==0);
    assert((data(&f)&H700_RG34XX_POWER_LED_PIN_MASK) && (data(&f)&(1u<<11)));
}
static void test_invalid_inputs_and_mux(void) {
    struct fake f={0}; h700_rg34xx_power_led_service_v1_t svc={0}; h700_rg34xx_power_led_io_v1_t io=io_for(&f);
    assert(h700_rg34xx_power_led_init_v1(NULL,sizeof(svc),&io)==H700_RG34XX_POWER_LED_ERR_ARGUMENT);
    assert(h700_rg34xx_power_led_init_v1(&svc,sizeof(svc)-1,&io)==H700_RG34XX_POWER_LED_ERR_ARGUMENT);
    assert(h700_rg34xx_power_led_set_game_active_v1(&svc,0)==H700_RG34XX_POWER_LED_ERR_NOT_READY);
    assert(h700_rg34xx_power_led_set_game_active_v1(&svc,2)==H700_RG34XX_POWER_LED_ERR_ARGUMENT);
    io.abi_version++; assert(h700_rg34xx_power_led_init_v1(&svc,sizeof(svc),&io)==H700_RG34XX_POWER_LED_ERR_ABI);
    io=io_for(&f); assert(h700_rg34xx_power_led_init_v1(&svc,sizeof(svc),&io)==0);
    f.regs[H700_RG34XX_POWER_LED_PIO_MUX1_OFFSET/4u]=0x77777777u;
    assert(h700_rg34xx_power_led_set_game_active_v1(&svc,1)==H700_RG34XX_POWER_LED_ERR_PINMUX);
}
static void test_physical_access_default_disabled(void) {
    assert(h700_rg34xx_power_led_early_init() == H700_RG34XX_POWER_LED_ERR_NOT_READY);
    assert(h700_rg34xx_power_led_is_ready() == 0);
    assert(h700_rg34xx_power_led_set_game_active(0u) == H700_RG34XX_POWER_LED_ERR_NOT_READY);
    assert(h700_rg34xx_power_led_set_game_active(1u) == H700_RG34XX_POWER_LED_ERR_NOT_READY);
    assert(h700_rg34xx_power_led_is_ready() == 0);
}
static void test_readback_failure(void) {
    struct fake f={0}; h700_rg34xx_power_led_service_v1_t svc={0}; h700_rg34xx_power_led_io_v1_t io=io_for(&f);
    f.drop=1; f.regs[H700_RG34XX_POWER_LED_PIO_MUX1_OFFSET/4u]=0x77777777u;
    assert(h700_rg34xx_power_led_init_v1(&svc,sizeof(svc),&io)==H700_RG34XX_POWER_LED_ERR_READBACK);
    assert(svc.initialized==0);
}
int main(void) {
    test_preload_and_bit_preservation(); test_game_lifecycle(); test_invalid_inputs_and_mux(); test_readback_failure(); test_physical_access_default_disabled();
    puts("RG34XX PI12 power LED lifecycle: PASS"); return 0;
}
