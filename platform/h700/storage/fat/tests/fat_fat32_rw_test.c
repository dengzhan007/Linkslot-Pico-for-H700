/* Host regression for FAT16/32 write paths on a real FAT32 image. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "fat_ro.h"
#include "fat_wr.h"

static FILE *g_img;
static unsigned img_read(void *ctx, uint32_t lba, uint32_t count, void *buf)
{
	(void)ctx;
	if (fseek(g_img, (long)lba * 512L, SEEK_SET) != 0) return 0;
	return (unsigned)fread(buf, 512, count, g_img);
}
static unsigned img_write(void *ctx, uint32_t lba, uint32_t count, const void *buf)
{
	(void)ctx;
	if (fseek(g_img, (long)lba * 512L, SEEK_SET) != 0) return 0;
	return (unsigned)fwrite(buf, 512, count, g_img);
}
static int fail(const char *why) { fprintf(stderr, "FAIL: %s\n", why); return 1; }

int main(int argc, char **argv)
{
	static unsigned char src[32768], got[32768];
	fat_vol v; fat_file f; fat_dir d; fat_dirent e;
	uint32_t first = 0, n; int i, count = 0;
	char too_long[80];
	if (argc != 2) return fail("usage: fat_fat32_rw_test image");
	g_img = fopen(argv[1], "r+b");
	if (!g_img) return fail("open image");
	if (fat_mount(&v, img_read, 0) != 0 || !v.is_fat32 || v.is_exfat)
		return fail("mount FAT32");
	fat_set_writer(&v, img_write);
	if (fat_wr_mkpath(&v, "/saves/gba") != 0) return fail("mkdir saves/gba");

	for (i = 0; i < (int)sizeof src; i++) src[i] = (unsigned char)(i * 17 + (i >> 8));
	if (fat_wr_put(&v, "/saves/gba", "Long Game Name (USA).sav", src, sizeof src) != 0)
		return fail("create VFAT long-name save");
	if (fat_open(&v, "/saves/gba/Long Game Name (USA).sav", &f) != 0)
		return fail("open created long-name save");
	n = fat_read(&f, 0, got, sizeof got);
	if (n != sizeof got || memcmp(src, got, sizeof src) != 0) return fail("created save content");

	for (i = 0; i < (int)sizeof src; i++) src[i] ^= 0x5A;
	if (fat_wr_put(&v, "/saves/gba", "Long Game Name (USA).sav", src, sizeof src) != 0)
		return fail("replace VFAT long-name save");
	if (fat_open(&v, "/saves/gba/Long Game Name (USA).sav", &f) != 0)
		return fail("open replaced save");
	n = fat_read(&f, 0, got, sizeof got);
	if (n != sizeof got || memcmp(src, got, sizeof src) != 0) return fail("replaced save content");

	if (fat_opendir(&v, "/saves/gba", &d) != 0) return fail("open saves directory");
	while (fat_readdir(&d, &e)) {
		if (!strcmp(e.name, "Long Game Name (USA).sav")) { count++; first = e.first_clus; }
	}
	if (count != 1 || first < 2) return fail("replacement left duplicate/missing directory entries");
	if (fat_wr_delete(&v, "/saves/gba", "Long Game Name (USA).sav", first) != 0)
		return fail("delete save");
	if (fat_open(&v, "/saves/gba/Long Game Name (USA).sav", &f) == 0)
		return fail("deleted save is still visible");

	if (fat_wr_put(&v, "/saves/gba", "empty.sav", 0, 0) != 0) return fail("create empty file");
	if (fat_open(&v, "/saves/gba/empty.sav", &f) != 0 || f.size != 0)
		return fail("read empty file metadata");

	too_long[0] = '/'; memset(too_long + 1, 'x', 64); too_long[65] = 0;
	if (fat_wr_mkpath(&v, too_long) == 0) return fail("reject overlong 8.3 directory component");
	fclose(g_img);
	puts("FAT32 writer regression PASS (LFN create/replace/delete, empty file, path rejection)");
	return 0;
}
