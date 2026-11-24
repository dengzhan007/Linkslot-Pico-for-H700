// Tiny uinput keypad for linkSlot device testing.
//
// Creates one virtual input device with the ANBERNIC key codes and a hat
// switch, then reads simple commands from a FIFO:
//
//   key 0x130 1      press A
//   key 0x130 0      release A
//   abs 0x10 -1      hold left
//   abs 0x10 0       release left
//   tap 0x130 120    press/release A for 120 ms
//   sleep 500
//   quit
//
// Build (arm64 container): gcc -O2 -o uinject uinject.c

#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/input.h>
#include <linux/uinput.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static int fd;

static void emit(int type, int code, int value) {
    struct input_event ev;
    memset(&ev, 0, sizeof(ev));
    ev.type = type;
    ev.code = code;
    ev.value = value;
    if (write(fd, &ev, sizeof(ev)) != (ssize_t)sizeof(ev)) {
        perror("write input_event");
    }
}

static void syn(void) { emit(EV_SYN, SYN_REPORT, 0); }

static void key(int code, int down) {
    emit(EV_KEY, code, down ? 1 : 0);
    syn();
}

static void hat(int code, int value) {
    emit(EV_ABS, code, value);
    syn();
}

static void enable_key(int code) { ioctl(fd, UI_SET_KEYBIT, code); }

static void setup_abs(int code) {
    struct uinput_abs_setup abs;
    memset(&abs, 0, sizeof(abs));
    abs.code = code;
    abs.absinfo.minimum = -1;
    abs.absinfo.maximum = 1;
    ioctl(fd, UI_ABS_SETUP, &abs);
    ioctl(fd, UI_SET_ABSBIT, code);
}

int main(int argc, char **argv) {
    const char *fifo = argc > 1 ? argv[1] : "/tmp/uinject.fifo";
    fd = open("/dev/uinput", O_WRONLY | O_NONBLOCK);
    if (fd < 0) {
        perror("open /dev/uinput");
        return 1;
    }

    ioctl(fd, UI_SET_EVBIT, EV_KEY);
    ioctl(fd, UI_SET_EVBIT, EV_ABS);
    ioctl(fd, UI_SET_EVBIT, EV_SYN);
    for (int code = 0x130; code <= 0x13b; code++) enable_key(code);
    enable_key(0x162);
    enable_key(KEY_VOLUMEUP);
    enable_key(KEY_VOLUMEDOWN);
    setup_abs(ABS_HAT0X);
    setup_abs(ABS_HAT0Y);

    struct uinput_setup setup;
    memset(&setup, 0, sizeof(setup));
    setup.id.bustype = BUS_USB;
    setup.id.vendor = 0x1234;
    setup.id.product = 0x5678;
    setup.id.version = 1;
    strncpy(setup.name, "linkSlot virtual pad", sizeof(setup.name) - 1);
    if (ioctl(fd, UI_DEV_SETUP, &setup) < 0) {
        perror("UI_DEV_SETUP");
        return 1;
    }
    if (ioctl(fd, UI_DEV_CREATE) < 0) {
        perror("UI_DEV_CREATE");
        return 1;
    }
    usleep(200000);
    printf("ready\n");
    fflush(stdout);

    int input = open(fifo, O_RDONLY | O_NONBLOCK);
    if (input < 0) {
        perror("open fifo");
        ioctl(fd, UI_DEV_DESTROY);
        return 1;
    }
    char pending[512] = {0};
    for (;;) {
        char chunk[512];
        ssize_t count = read(input, chunk, sizeof(chunk) - 1);
        if (count > 0) {
            chunk[count] = 0;
            strncat(pending, chunk, sizeof(pending) - strlen(pending) - 1);
        }
        char *newline;
        while ((newline = strchr(pending, '\n')) != NULL) {
            *newline = 0;
            char command[32];
            unsigned int code = 0;
            int value = 0;
            if (sscanf(pending, "%31s %i %i", command, (int *)&code, &value) >= 1) {
                if (strcmp(command, "key") == 0) {
                    key((int)code, value);
                } else if (strcmp(command, "abs") == 0) {
                    hat((int)code, value);
                } else if (strcmp(command, "tap") == 0) {
                    key((int)code, 1);
                    usleep((useconds_t)value * 1000);
                    key((int)code, 0);
                } else if (strcmp(command, "sleep") == 0) {
                    usleep((useconds_t)code * 1000);
                } else if (strcmp(command, "quit") == 0) {
                    ioctl(fd, UI_DEV_DESTROY);
                    close(fd);
                    return 0;
                }
            }
            memmove(pending, newline + 1, strlen(newline + 1) + 1);
        }
        usleep(50000);
    }
    return 0;
}
