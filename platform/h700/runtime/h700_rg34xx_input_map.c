#include "h700_rg34xx_input_map.h"
_Static_assert(H700_RG34XX_INPUT_MAP_COUNT==17u,"RG34XX live-FDT map must contain 17 GPIO inputs");
const h700_rg34xx_input_map_entry_t h700_rg34xx_input_map[H700_RG34XX_INPUT_MAP_COUNT]={
 {H700_RG34XX_GPIO_PORT_A,6u,0u,1u,2u,1u,H700_RG34XX_EV_ABS,0u,17u,-1,H700_BTN_UP},
 {H700_RG34XX_GPIO_PORT_E,0u,0u,1u,2u,1u,H700_RG34XX_EV_ABS,0u,17u,1,H700_BTN_DOWN},
 {H700_RG34XX_GPIO_PORT_A,8u,0u,1u,2u,1u,H700_RG34XX_EV_ABS,0u,16u,-1,H700_BTN_LEFT},
 {H700_RG34XX_GPIO_PORT_A,9u,0u,1u,2u,1u,H700_RG34XX_EV_ABS,0u,16u,1,H700_BTN_RIGHT},
 {H700_RG34XX_GPIO_PORT_A,0u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,304u,1,H700_BTN_A},
 {H700_RG34XX_GPIO_PORT_A,1u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,305u,1,H700_BTN_B},
 /* Physical silk X is wired to PA2 and emits the DT keyY event code 307. */
 {H700_RG34XX_GPIO_PORT_A,2u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,307u,1,H700_BTN_X},
 /* Physical silk Y is wired to PA3 and emits the DT keyX event code 306. */
 {H700_RG34XX_GPIO_PORT_A,3u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,306u,1,H700_BTN_Y},
 {H700_RG34XX_GPIO_PORT_A,10u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,308u,1,H700_BTN_L1},
 {H700_RG34XX_GPIO_PORT_A,11u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,314u,1,H700_BTN_L2},
 {H700_RG34XX_GPIO_PORT_A,12u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,309u,1,H700_BTN_R1},
 {H700_RG34XX_GPIO_PORT_A,7u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,315u,1,H700_BTN_R2},
 {H700_RG34XX_GPIO_PORT_A,4u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,311u,1,H700_BTN_START},
 {H700_RG34XX_GPIO_PORT_A,5u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,310u,1,H700_BTN_SELECT},
 {H700_RG34XX_GPIO_PORT_E,3u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,312u,1,H700_BTN_MENU},
 {H700_RG34XX_GPIO_PORT_E,1u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,115u,1,H700_BTN_VOL_UP},
 {H700_RG34XX_GPIO_PORT_E,2u,0u,1u,2u,1u,H700_RG34XX_EV_KEY,0u,114u,1,H700_BTN_VOL_DOWN}
};
void h700_rg34xx_input_state_init(h700_rg34xx_input_state_t *state){
 if(!state)return;
 state->previous_pressed_mask=0u;state->initialized=false;
}
size_t h700_rg34xx_input_update(h700_rg34xx_input_state_t *state,h700_input_queue_t *queue,
 uint32_t port_a_levels,uint32_t port_e_levels){
 if(!state||!queue)return 0u;
 uint32_t current=0u;
 for(size_t i=0u;i<H700_RG34XX_INPUT_MAP_COUNT;i++){
  const h700_rg34xx_input_map_entry_t *e=&h700_rg34xx_input_map[i];
  uint32_t levels=e->port==H700_RG34XX_GPIO_PORT_A?port_a_levels:port_e_levels;
  bool high=((levels>>e->pin)&1u)!=0u;
  if(e->active_low?!high:high)current|=(uint32_t)1u<<i;
 }
 uint32_t changed=state->initialized?(state->previous_pressed_mask^current):current;
 size_t pushed=0u;
 for(size_t i=0u;i<H700_RG34XX_INPUT_MAP_COUNT;i++){
  uint32_t mask=(uint32_t)1u<<i;
  if(!(changed&mask))continue;
  uint8_t edge=(current&mask)?H700_INPUT_PRESSED:H700_INPUT_RELEASED;
  if(h700_input_queue_push(queue,h700_rg34xx_input_map[i].logical_button,edge))pushed++;
 }
 state->previous_pressed_mask=current;state->initialized=true;
 return pushed;
}
