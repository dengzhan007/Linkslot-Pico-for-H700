#include <assert.h>
#include <stdio.h>
#include "h700_rg34xx_input_map.h"
static const h700_rg34xx_input_map_entry_t *find_button(uint16_t b){
 for(size_t i=0;i<H700_RG34XX_INPUT_MAP_COUNT;i++)if(h700_rg34xx_input_map[i].logical_button==b)return &h700_rg34xx_input_map[i];
 return NULL;
}
static size_t drain(h700_input_queue_t *q,h700_input_event_t *e,size_t n){return h700_input_queue_drain(q,e,n);}
static void test_map(void){
 assert(H700_RG34XX_INPUT_DECLARED_POLL_INTERVAL_MS==20u);
 for(size_t i=0;i<H700_RG34XX_INPUT_MAP_COUNT;i++){
  const h700_rg34xx_input_map_entry_t *e=&h700_rg34xx_input_map[i];
  assert(e->port==H700_RG34XX_GPIO_PORT_A||e->port==H700_RG34XX_GPIO_PORT_E);
  assert(e->pin<32u&&e->mux_select==0u&&e->pull_config==1u&&e->drive_level==2u&&e->active_low==1u);
  assert(e->input_type==H700_RG34XX_EV_KEY||e->input_type==H700_RG34XX_EV_ABS);
  assert(e->logical_button>=H700_BTN_UP&&e->logical_button<=H700_BTN_VOL_DOWN);
  for(size_t j=i+1;j<H700_RG34XX_INPUT_MAP_COUNT;j++){
   assert(e->logical_button!=h700_rg34xx_input_map[j].logical_button);
   assert(e->port!=h700_rg34xx_input_map[j].port||e->pin!=h700_rg34xx_input_map[j].pin);
  }
 }
 assert(find_button(H700_BTN_A)&&find_button(H700_BTN_A)->pin==0u);
 assert(find_button(H700_BTN_MENU)&&find_button(H700_BTN_MENU)->port==H700_RG34XX_GPIO_PORT_E&&find_button(H700_BTN_MENU)->pin==3u);
 assert(find_button(H700_BTN_MENU)->linux_code==312u);
 assert(find_button(H700_BTN_L2)->linux_code==314u&&find_button(H700_BTN_R2)->linux_code==315u);
}
static void test_edges(void){
 h700_input_queue_t q;h700_rg34xx_input_state_t s;h700_input_event_t ev[H700_INPUT_QUEUE_CAPACITY];
 h700_input_queue_init(&q);h700_rg34xx_input_state_init(&s);
 assert(h700_rg34xx_input_update(&s,&q,UINT32_MAX,UINT32_MAX)==0u);
 uint32_t a=UINT32_MAX&~1u;
 assert(h700_rg34xx_input_update(&s,&q,a,UINT32_MAX)==1u);
 assert(drain(&q,ev,H700_INPUT_QUEUE_CAPACITY)==1u&&ev[0].button==H700_BTN_A&&ev[0].state==H700_INPUT_PRESSED);
 assert(h700_rg34xx_input_update(&s,&q,a,UINT32_MAX)==0u);
 assert(h700_rg34xx_input_update(&s,&q,UINT32_MAX,UINT32_MAX)==1u);
 assert(drain(&q,ev,H700_INPUT_QUEUE_CAPACITY)==1u&&ev[0].button==H700_BTN_A&&ev[0].state==H700_INPUT_RELEASED);
}
static void test_initial_held_and_dpad(void){
 h700_input_queue_t q;h700_rg34xx_input_state_t s;h700_input_event_t ev[H700_INPUT_QUEUE_CAPACITY];
 h700_input_queue_init(&q);h700_rg34xx_input_state_init(&s);
 uint32_t e=UINT32_MAX&~(1u<<3);
 assert(h700_rg34xx_input_update(&s,&q,UINT32_MAX,e)==1u);
 assert(drain(&q,ev,H700_INPUT_QUEUE_CAPACITY)==1u&&ev[0].button==H700_BTN_MENU&&ev[0].state==H700_INPUT_PRESSED);
 h700_input_queue_init(&q);h700_rg34xx_input_state_init(&s);
 uint32_t a=UINT32_MAX&~((1u<<6)|(1u<<8)|(1u<<9));e=UINT32_MAX&~(1u<<0);
 assert(h700_rg34xx_input_update(&s,&q,a,e)==4u);
 assert(drain(&q,ev,H700_INPUT_QUEUE_CAPACITY)==4u);
 assert(ev[0].button==H700_BTN_UP&&ev[1].button==H700_BTN_DOWN&&ev[2].button==H700_BTN_LEFT&&ev[3].button==H700_BTN_RIGHT);
 for(size_t i=0;i<4;i++)assert(ev[i].state==H700_INPUT_PRESSED);
}
static void test_unmapped_and_all(void){
 h700_input_queue_t q;h700_rg34xx_input_state_t s;h700_input_event_t ev[H700_INPUT_QUEUE_CAPACITY];
 h700_input_queue_init(&q);h700_rg34xx_input_state_init(&s);
 assert(h700_rg34xx_input_update(&s,&q,UINT32_MAX&~(1u<<31),UINT32_MAX&~(1u<<31))==0u);
 assert(h700_rg34xx_input_update(&s,&q,0u,0u)==H700_RG34XX_INPUT_MAP_COUNT);
 assert(drain(&q,ev,H700_INPUT_QUEUE_CAPACITY)==H700_RG34XX_INPUT_MAP_COUNT);
 for(size_t i=0;i<H700_RG34XX_INPUT_MAP_COUNT;i++)assert(ev[i].state==H700_INPUT_PRESSED);
 assert(h700_rg34xx_input_update(&s,&q,UINT32_MAX,UINT32_MAX)==H700_RG34XX_INPUT_MAP_COUNT);
 assert(drain(&q,ev,H700_INPUT_QUEUE_CAPACITY)==H700_RG34XX_INPUT_MAP_COUNT);
 for(size_t i=0;i<H700_RG34XX_INPUT_MAP_COUNT;i++)assert(ev[i].state==H700_INPUT_RELEASED);
}
static void test_nulls(void){
 h700_rg34xx_input_state_t s;h700_input_queue_t q;h700_input_queue_init(&q);h700_rg34xx_input_state_init(&s);
 h700_rg34xx_input_state_init(NULL);
 assert(h700_rg34xx_input_update(NULL,&q,0,0)==0u&&h700_rg34xx_input_update(&s,NULL,0,0)==0u);
}
int main(void){test_map();test_edges();test_initial_held_and_dpad();test_unmapped_and_all();test_nulls();puts("H700 RG34XX input map host tests passed.");return 0;}
