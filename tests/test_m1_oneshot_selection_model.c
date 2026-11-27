#include "m1_oneshot_selection_model.h"

#include <stdio.h>
#include <string.h>

static uint8_t stock_bytes[H700_M1_STOCK_BOOT0_BYTES];
static uint8_t m1_bytes[H700_M1_SPL_IMAGE_BYTES];

static void fill_image(uint8_t *bytes, size_t size, uint8_t seed)
{
    for (size_t i = 0; i < size; ++i)
        bytes[i] = (uint8_t)(seed + (uint8_t)(i * 37u));
    memcpy(bytes + 4u, "eGON.BT0", 8u);
}

static int expect(enum h700_m1_capsule_choice got,
                  enum h700_m1_capsule_choice expected,
                  const char *name)
{
    if (got != expected) {
        fprintf(stderr, "FAIL %s: got=%d expected=%d\n",
                name, (int)got, (int)expected);
        return 0;
    }
    printf("PASS %s\n", name);
    return 1;
}

int main(void)
{
    struct h700_m1_capsule_image stock = {
        stock_bytes, sizeof(stock_bytes), 0
    };
    struct h700_m1_capsule_image m1 = {
        m1_bytes, sizeof(m1_bytes), 0
    };
    int passed = 0;

    fill_image(stock_bytes, sizeof(stock_bytes), 0x31u);
    fill_image(m1_bytes, sizeof(m1_bytes), 0x72u);
    stock.expected_fnv1a =
        h700_m1_capsule_fnv1a(stock.bytes, stock.size);
    m1.expected_fnv1a =
        h700_m1_capsule_fnv1a(m1.bytes, m1.size);

    passed += expect(h700_m1_capsule_select(&stock, &m1),
                     H700_M1_CAPSULE_SPL,
                     "valid stock and valid M1 SPL select M1");

    m1_bytes[128] ^= 0x80u;
    passed += expect(h700_m1_capsule_select(&stock, &m1),
                     H700_M1_CAPSULE_STOCK_BOOT0,
                     "M1 SPL hash mismatch selects stock");
    m1_bytes[128] ^= 0x80u;

    m1_bytes[4] = 'X';
    passed += expect(h700_m1_capsule_select(&stock, &m1),
                     H700_M1_CAPSULE_STOCK_BOOT0,
                     "M1 SPL eGON mismatch selects stock");
    m1_bytes[4] = 'e';

    m1.size--;
    passed += expect(h700_m1_capsule_select(&stock, &m1),
                     H700_M1_CAPSULE_STOCK_BOOT0,
                     "M1 SPL size mismatch selects stock");
    m1.size = sizeof(m1_bytes);

    stock_bytes[64] ^= 0x01u;
    passed += expect(h700_m1_capsule_select(&stock, &m1),
                     H700_M1_CAPSULE_INVALID,
                     "invalid stock Boot0 fail-stops even with valid M1 SPL");

    passed += expect(h700_m1_capsule_select(0, &m1),
                     H700_M1_CAPSULE_INVALID,
                     "missing stock candidate fail-stops");

    printf("M1 capsule selection model: %d/6 cases passed\n", passed);
    return passed == 6 ? 0 : 1;
}
