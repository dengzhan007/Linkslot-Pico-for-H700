/*
 * Compile-time layout probe for the pinned armGBA committed-frame counter.
 *
 * The game counter is GBA::video.frameCounter. This probe intentionally does
 * not expose or modify emulator state; it only verifies the source ABI layout.
 */
#include <stddef.h>
#include <stdint.h>
#include "gba.h"

#if defined(ARMGBA_PRINT_LAYOUT)
#include <stdio.h>

int main(void) {
    const size_t video_offset = offsetof(struct GBA, video);
    const size_t counter_offset = offsetof(struct Video, frameCounter);
    printf("gba_size=%zu\n", sizeof(struct GBA));
    printf("video_offset=%zu\n", video_offset);
    printf("frame_counter_offset=%zu\n", counter_offset);
    printf("gba_frame_counter_offset=%zu\n", video_offset + counter_offset);
    return 0;
}
#else
#ifndef ARMGBA_EXPECT_GBA_SIZE
#error "ARMGBA_EXPECT_GBA_SIZE must be provided for cross-layout checks"
#endif
#ifndef ARMGBA_EXPECT_VIDEO_OFFSET
#error "ARMGBA_EXPECT_VIDEO_OFFSET must be provided for cross-layout checks"
#endif
#ifndef ARMGBA_EXPECT_COUNTER_OFFSET
#error "ARMGBA_EXPECT_COUNTER_OFFSET must be provided for cross-layout checks"
#endif
#ifndef ARMGBA_EXPECT_FRAME_OFFSET
#error "ARMGBA_EXPECT_FRAME_OFFSET must be provided for cross-layout checks"
#endif

_Static_assert(sizeof(struct GBA) == ARMGBA_EXPECT_GBA_SIZE,
               "armGBA GBA struct size differs from the host layout");
_Static_assert(offsetof(struct GBA, video) == ARMGBA_EXPECT_VIDEO_OFFSET,
               "armGBA video field offset differs from the host layout");
_Static_assert(offsetof(struct Video, frameCounter) == ARMGBA_EXPECT_COUNTER_OFFSET,
               "armGBA frameCounter field offset differs from the host layout");
_Static_assert(offsetof(struct GBA, video) + offsetof(struct Video, frameCounter) ==
                   ARMGBA_EXPECT_FRAME_OFFSET,
               "armGBA committed-frame offset differs from the host layout");

/* Keep the verified values present in the object for artifact inspection. */
__attribute__((used, section(".rodata.armgba_layout")))
const uint32_t armgba_layout_probe_values[] = {
    (uint32_t)sizeof(struct GBA),
    (uint32_t)offsetof(struct GBA, video),
    (uint32_t)offsetof(struct Video, frameCounter),
    (uint32_t)(offsetof(struct GBA, video) + offsetof(struct Video, frameCounter)),
};
#endif
