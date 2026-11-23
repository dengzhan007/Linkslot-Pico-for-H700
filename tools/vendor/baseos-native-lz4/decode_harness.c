#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "lz4-stream.h"

static unsigned char *read_all(const char *path, unsigned long *length)
{
    FILE *file = fopen(path, "rb");
    long size;
    unsigned char *buffer;

    if (file == NULL || fseek(file, 0, SEEK_END) != 0) {
        if (file != NULL) fclose(file);
        return NULL;
    }
    size = ftell(file);
    if (size < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return NULL;
    }
    buffer = (unsigned char *)malloc((size_t)size);
    if (buffer == NULL) {
        fclose(file);
        return NULL;
    }
    if (fread(buffer, 1u, (size_t)size, file) != (size_t)size) {
        free(buffer);
        fclose(file);
        return NULL;
    }
    fclose(file);
    *length = (unsigned long)size;
    return buffer;
}

int main(int argc, char **argv)
{
    struct baseos_lz4 state;
    unsigned long source_length = 0u;
    unsigned char *source;
    unsigned char *destination;
    unsigned long capacity = 0x01f80000ul;
    int result;
    FILE *output;

    if (argc != 3) return 2;
    source = read_all(argv[1], &source_length);
    if (source == NULL) return 3;
    destination = (unsigned char *)malloc((size_t)capacity);
    if (destination == NULL) {
        free(source);
        return 4;
    }
    result = baseos_lz4_begin(&state, destination, capacity, source, source_length);
    if (result != 0) {
        free(destination);
        free(source);
        return 5;
    }
    do {
        result = baseos_lz4_step(&state);
    } while (result == 1);
    if (result != 0) {
        free(destination);
        free(source);
        return 6;
    }
    output = fopen(argv[2], "wb");
    if (output == NULL ||
        fwrite(destination, 1u, (size_t)(state.out - state.dst), output) !=
            (size_t)(state.out - state.dst)) {
        if (output != NULL) fclose(output);
        free(destination);
        free(source);
        return 7;
    }
    fclose(output);
    free(destination);
    free(source);
    return 0;
}
