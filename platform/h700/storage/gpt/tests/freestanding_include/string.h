#ifndef H700_GPT_FREESTANDING_STRING_H
#define H700_GPT_FREESTANDING_STRING_H

#include <stddef.h>

/* The AArch64 compile probe only needs declarations; LK supplies the symbols. */
void *memcpy(void *destination, const void *source, size_t length);
void *memset(void *destination, int value, size_t length);
int memcmp(const void *left, const void *right, size_t length);
int strcmp(const char *left, const char *right);

#endif /* H700_GPT_FREESTANDING_STRING_H */
