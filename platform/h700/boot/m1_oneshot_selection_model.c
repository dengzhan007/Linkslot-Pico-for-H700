#include "m1_oneshot_selection_model.h"

static int egon_image_valid(const struct h700_m1_capsule_image *image,
                            size_t expected_size)
{
    static const uint8_t egon_magic[8] = {
        'e', 'G', 'O', 'N', '.', 'B', 'T', '0'
    };

    if (image == 0 || image->bytes == 0 ||
        image->size != expected_size || image->size < 12u)
        return 0;

    for (size_t i = 0; i < sizeof(egon_magic); ++i) {
        if (image->bytes[4u + i] != egon_magic[i])
            return 0;
    }

    return h700_m1_capsule_fnv1a(image->bytes, image->size) ==
           image->expected_fnv1a;
}

uint32_t h700_m1_capsule_fnv1a(const uint8_t *bytes, size_t size)
{
    uint32_t hash = UINT32_C(2166136261);

    if (bytes == 0)
        return 0;

    for (size_t i = 0; i < size; ++i) {
        hash ^= bytes[i];
        hash *= UINT32_C(16777619);
    }

    return hash;
}

enum h700_m1_capsule_choice h700_m1_capsule_select(
    const struct h700_m1_capsule_image *stock,
    const struct h700_m1_capsule_image *m1_spl)
{
    /* No M1 path is allowed unless the embedded recovery Boot0 is trusted. */
    if (!egon_image_valid(stock, H700_M1_STOCK_BOOT0_BYTES))
        return H700_M1_CAPSULE_INVALID;

    if (egon_image_valid(m1_spl, H700_M1_SPL_IMAGE_BYTES))
        return H700_M1_CAPSULE_SPL;

    return H700_M1_CAPSULE_STOCK_BOOT0;
}
