#include "native-attempt.h"

#include <stdio.h>
#include <string.h>

struct fake_rtc {
    uint32_t words[2];
    unsigned int writes;
    unsigned int fences;
    unsigned int fail_write_at;
};

static uint32_t fake_read(void *context, enum attempt_word word)
{
    struct fake_rtc *rtc = context;
    return rtc->words[(unsigned int)word];
}

static void fake_write(void *context, enum attempt_word word, uint32_t value)
{
    struct fake_rtc *rtc = context;
    rtc->writes++;
    if (rtc->fail_write_at && rtc->writes == rtc->fail_write_at)
        return;
    rtc->words[(unsigned int)word] = value;
}

static void fake_fence(void *context)
{
    struct fake_rtc *rtc = context;
    rtc->fences++;
}

static struct attempt_io io_for(struct fake_rtc *rtc)
{
    struct attempt_io io = { rtc, fake_read, fake_write, fake_fence };
    return io;
}

static int check(int condition, const char *name)
{
    if (!condition) {
        fprintf(stderr, "FAIL %s\n", name);
        return 0;
    }
    printf("PASS %s\n", name);
    return 1;
}

int main(void)
{
    static const uint8_t generation[32] = {
        0x01,0x23,0x45,0x67,0x89,0xab,0xcd,0xef,
        0x10,0x32,0x54,0x76,0x98,0xba,0xdc,0xfe,
        0x55,0xaa,0x55,0xaa,0x11,0x22,0x33,0x44,
        0xde,0xad,0xbe,0xef,0xca,0xfe,0xba,0xbe
    };
    static const uint8_t other_generation[32] = {
        0x02,0x23,0x45,0x67,0x89,0xab,0xcd,0xef,
        0x10,0x32,0x54,0x76,0x98,0xba,0xdc,0xfe,
        0x55,0xaa,0x55,0xaa,0x11,0x22,0x33,0x44,
        0xde,0xad,0xbe,0xef,0xca,0xfe,0xba,0xbe
    };
    const uint32_t cookie = generation_cookie(generation);
    int passed = 0;
    struct fake_rtc rtc = {{0,0},0,0,0};
    struct attempt_io io = io_for(&rtc);

    passed += check(cookie != 0, "generation cookie is nonzero");
    passed += check(boot_plan((struct attempt_pair){0,0}, 0, cookie) == NATIVE_ATTEMPT,
                    "empty pair permits native attempt");
    passed += check(boot_plan((struct attempt_pair){0,0}, 1, cookie) == STOCK_RESUME,
                    "resume takes precedence over empty pair");
    passed += check(boot_plan((struct attempt_pair){cookie,0}, 0, cookie) == STOCK_RECOVERY,
                    "staged cookie fails closed to stock recovery");
    passed += check(boot_plan((struct attempt_pair){0,ATTEMPT_PENDING}, 0, cookie) == STOCK_RECOVERY,
                    "orphan marker fails closed to stock recovery");
    passed += check(boot_plan((struct attempt_pair){0,0}, 0, 0) == INVALID_GENERATION,
                    "zero generation cookie is invalid");

    passed += check(attempt_commit(&io, cookie) == IO_COMPLETE,
                    "attempt commit writes ordered cookie then marker");
    passed += check(rtc.words[ATTEMPT_COOKIE] == cookie &&
                    rtc.words[ATTEMPT_MARKER] == ATTEMPT_PENDING &&
                    rtc.writes == 2 && rtc.fences >= 3,
                    "commit readback and barriers leave pending pair");

    rtc.writes = 0;
    rtc.fences = 0;
    rtc.fail_write_at = 1;
    rtc.words[0] = rtc.words[1] = 0;
    passed += check(attempt_commit(&io, cookie) == IO_CHANGED,
                    "dropped cookie write is detected");
    passed += check(boot_plan(pair_read(&io), 0, cookie) == NATIVE_ATTEMPT,
                    "dropped first write leaves empty retryable pair");

    rtc.writes = 0;
    rtc.fences = 0;
    rtc.fail_write_at = 2;
    rtc.words[0] = rtc.words[1] = 0;
    passed += check(attempt_commit(&io, cookie) == IO_CHANGED,
                    "dropped marker write is detected");
    passed += check(boot_plan(pair_read(&io), 0, cookie) == STOCK_RECOVERY,
                    "torn second write fails closed to stock recovery");

    rtc.fail_write_at = 0;
    rtc.words[0] = rtc.words[1] = 0;
    rtc.writes = rtc.fences = 0;
    passed += check(attempt_commit(&io, cookie) == IO_COMPLETE,
                    "fresh commit before acknowledgment tests");

    passed += check(attempt_ack(&io, 0, generation, generation) == IO_REFUSED,
                    "health failure refuses acknowledgment");
    passed += check(attempt_ack(&io, 1, other_generation, generation) == IO_REFUSED,
                    "generation mismatch refuses acknowledgment");

    rtc.writes = 0;
    rtc.fences = 0;
    passed += check(attempt_ack(&io, 1, generation, generation) == IO_COMPLETE,
                    "healthy matching generation clears attempt pair");
    passed += check(rtc.words[0] == 0 && rtc.words[1] == 0,
                    "acknowledgment leaves empty pair");

    rtc.words[0] = cookie;
    rtc.words[1] = ATTEMPT_PENDING;
    rtc.writes = rtc.fences = 0;
    rtc.fail_write_at = 2;
    passed += check(attempt_ack(&io, 1, generation, generation) == IO_CHANGED,
                    "torn acknowledgment is detected");
    passed += check(boot_plan(pair_read(&io), 0, cookie) == STOCK_RECOVERY,
                    "torn acknowledgment fails closed to stock recovery");

    printf("BaseOS RTC attempt-pair host model: %d/17 checks passed\n", passed);
    return passed == 19 ? 0 : 1;
}
