/* A flat C ABI over libmgba's `struct mCore`.
 *
 * The point of this file is that `struct mCore` is a ~90-entry table of function
 * pointers with four structs embedded by value, and its layout shifts with the
 * USE_DEBUGGERS / MINIMAL_CORE / ENABLE_SCRIPTING flags the library was built
 * with. Transcribing that into Rust by hand would be a silent-corruption bug
 * waiting to happen. Compiling this shim against the *installed* headers means
 * the C compiler derives the layout from the same flags.h the .so was built
 * with, and Rust only ever sees the flat functions below.
 */

/* First on purpose: 0.11's common.h no longer includes flags.h (the mGBA build
 * passes these as -D flags instead), but the installed flags.h still records
 * what the .so was built with -- ENABLE_VFS in particular, which gates the
 * VFileOpen declaration in mgba-util/vfs.h. */
#include <mgba/flags.h>

/* flags.h lies about exactly one flag at commit 94b1578f: CMakeLists.txt:869
 * does `list(APPEND ENABLES VFS DIRECTORIES)` whenever ENABLE_VFS is on, so the
 * library is compiled with -DENABLE_DIRECTORIES -- but no cmake *variable* of
 * that name ever exists, so flags.h's `#cmakedefine ENABLE_DIRECTORIES` stays
 * undefined. The flag gates `struct mDirectorySet dirs` (4152 bytes) embedded
 * in struct mCore ahead of every function pointer, so omitting it shifts the
 * entire vtable and the first call lands on a NULL slot. Found the hard way;
 * verified by dumping the real vtable offset (4856) against offsetof. */
#if defined(ENABLE_VFS) && !defined(ENABLE_DIRECTORIES)
#define ENABLE_DIRECTORIES
#endif

#include <mgba/core/core.h>
#include <mgba/core/interface.h>
#include <mgba/core/log.h>
#include <mgba/core/serialize.h>
#include <mgba/gba/core.h>
#include <mgba-util/vfs.h>

#include <fcntl.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

struct FrlgCore {
	struct mCore* core;
	mColor* video;
	unsigned width;
	unsigned height;
};

void frlg_core_free(struct FrlgCore* h);

/* --- logging ------------------------------------------------------------ */

/* Without this the core writes DMA and BIOS-call chatter to stdout, which
 * drowns any harness that reports on stdout. */

static void frlg_log_discard(struct mLogger* logger, int category,
                             enum mLogLevel level, const char* format,
                             va_list args) {
	(void) logger;
	(void) category;
	(void) level;
	(void) format;
	(void) args;
}

static struct mLogFilter frlg_filter;
static struct mLogger frlg_logger;
static int frlg_logger_ready = 0;

void frlg_silence_logs(void) {
	if (frlg_logger_ready) {
		return;
	}
	mLogFilterInit(&frlg_filter);
	frlg_filter.defaultLevels = 0;
	frlg_logger.log = frlg_log_discard;
	frlg_logger.filter = &frlg_filter;
	mLogSetDefaultLogger(&frlg_logger);
	frlg_logger_ready = 1;
}

/* --- lifecycle ---------------------------------------------------------- */

struct FrlgCore* frlg_core_new(const char* rom_path) {
	struct FrlgCore* h = calloc(1, sizeof(*h));
	if (!h) {
		return NULL;
	}

	h->core = GBACoreCreate();
	if (!h->core) {
		free(h);
		return NULL;
	}
	if (!h->core->init(h->core)) {
		h->core->deinit(h->core);
		free(h);
		return NULL;
	}

	mCoreInitConfig(h->core, NULL);

	h->core->baseVideoSize(h->core, &h->width, &h->height);
	h->video = calloc((size_t) h->width * h->height, sizeof(mColor));
	if (!h->video) {
		mCoreConfigDeinit(&h->core->config);
		h->core->deinit(h->core);
		free(h);
		return NULL;
	}
	h->core->setVideoBuffer(h->core, h->video, h->width);

	struct VFile* vf = VFileOpen(rom_path, O_RDONLY);
	if (!vf) {
		frlg_core_free(h);
		return NULL;
	}
	/* loadROM takes ownership of vf on success; on failure it is ours. */
	if (!h->core->loadROM(h->core, vf)) {
		vf->close(vf);
		frlg_core_free(h);
		return NULL;
	}

	h->core->reset(h->core);
	return h;
}

/* Optional: a real GBA BIOS. mGBA falls back to an HLE BIOS when none is
 * loaded, and HLE-vs-real is a real divergence axis against BizHawk: its SWI
 * handlers are not cycle-identical. Must be called before the first frame; it
 * resets the core.
 *
 * skip_intro chooses whether the boot animation runs. BizHawk *movie playback*
 * always runs it: MGBAHawk.cs:41 (2.11.1) passes
 * `skipBios: _syncSettings.SkipBios && !lp.DeterministicEmulationRequested`,
 * and loading a movie requests deterministic emulation (that is what makes
 * line 30's MissingFirmwareException fire without a BIOS -- observed on the
 * host, 2026-08-11). So the SyncSettings' `SkipBios: true` is overridden to
 * false whenever a .bk2 is replayed, its glue never calls GBASkipBIOS
 * (bizinterface.c:171, gated on ctx->skipbios), and the ~190-frame intro runs
 * with movie input already being fed. A tier-1 boot that skips the intro is
 * therefore shifted against tier 2 by the whole intro -- which is exactly the
 * desync the first watched replay showed. Pass skip_intro = 0 to match a
 * movie replay; 1 exists for interactive experiments only. */
int frlg_core_load_bios(struct FrlgCore* h, const char* bios_path,
                        int skip_intro) {
	struct VFile* vf = VFileOpen(bios_path, O_RDONLY);
	if (!vf) {
		return 0;
	}
	if (!h->core->loadBIOS(h->core, vf, 0)) {
		vf->close(vf);
		return 0;
	}
	/* opts.skipBios routes _GBACoreReset through the same GBASkipBIOS call
	 * BizHawk's glue makes when it does skip. */
	h->core->opts.skipBios = skip_intro ? true : false;
	h->core->reset(h->core);
	return 1;
}

void frlg_core_free(struct FrlgCore* h) {
	if (!h) {
		return;
	}
	if (h->core) {
		mCoreConfigDeinit(&h->core->config);
		h->core->deinit(h->core);
	}
	free(h->video);
	free(h);
}

void frlg_core_reset(struct FrlgCore* h) {
	h->core->reset(h->core);
}

/* --- running ------------------------------------------------------------ */

void frlg_run_frame(struct FrlgCore* h, uint16_t keys) {
	h->core->setKeys(h->core, keys);
	h->core->runFrame(h->core);
}

uint32_t frlg_frame_counter(const struct FrlgCore* h) {
	return h->core->frameCounter(h->core);
}

/* --- memory ------------------------------------------------------------- */

uint32_t frlg_read8(struct FrlgCore* h, uint32_t addr) {
	return h->core->busRead8(h->core, addr);
}

uint32_t frlg_read16(struct FrlgCore* h, uint32_t addr) {
	return h->core->busRead16(h->core, addr);
}

uint32_t frlg_read32(struct FrlgCore* h, uint32_t addr) {
	return h->core->busRead32(h->core, addr);
}

void frlg_read_range(struct FrlgCore* h, uint32_t addr, uint8_t* out,
                     size_t len) {
	for (size_t i = 0; i < len; ++i) {
		out[i] = (uint8_t) h->core->busRead8(h->core, addr + (uint32_t) i);
	}
}

void frlg_write8(struct FrlgCore* h, uint32_t addr, uint8_t value) {
	h->core->busWrite8(h->core, addr, value);
}

/* Direct pointer into the memory block containing `addr`, for bulk scans that
 * would be slow one busRead8 at a time. `*offset_out` is where `addr` lands in
 * the returned block and `*size_out` is the block's total size. */
void* frlg_memory_block(struct FrlgCore* h, uint32_t addr, size_t* size_out,
                        uint32_t* offset_out) {
	const struct mCoreMemoryBlock* blocks = NULL;
	size_t n = h->core->listMemoryBlocks(h->core, &blocks);

	/* The GBA list opens with a catch-all "mem" block spanning 0-0x10000000
	 * that has no backing pointer, so "first block containing addr" is always
	 * wrong. Take the narrowest block that both contains addr and has real
	 * storage behind it. */
	const struct mCoreMemoryBlock* best = NULL;
	void* best_base = NULL;
	size_t best_size = 0;

	for (size_t i = 0; i < n; ++i) {
		const struct mCoreMemoryBlock* b = &blocks[i];
		if (addr < b->start || addr >= b->end) {
			continue;
		}
		if (best && (b->end - b->start) >= (best->end - best->start)) {
			continue;
		}
		size_t size = 0;
		void* base = h->core->getMemoryBlock(h->core, b->id, &size);
		if (!base || size == 0) {
			continue;
		}
		best = b;
		best_base = base;
		best_size = size;
	}

	if (!best) {
		return NULL;
	}
	*size_out = best_size;
	*offset_out = (uint32_t) ((addr - best->start) % best_size);
	return best_base;
}

/* --- savestates --------------------------------------------------------- */

/* The raw core state: fixed size, no extdata, no savedata. This is the one for
 * the inner search loop -- save, try inputs, restore. */

size_t frlg_state_size(struct FrlgCore* h) {
	return h->core->stateSize(h->core);
}

int frlg_state_save(struct FrlgCore* h, void* buf) {
	return h->core->saveState(h->core, buf) ? 1 : 0;
}

int frlg_state_load(struct FrlgCore* h, const void* buf) {
	return h->core->loadState(h->core, buf) ? 1 : 0;
}

/* The full serialized state, including savedata and RTC: this is what a route
 * checkpoint on disk should be, since it survives without the SRAM alongside. */

/* O_RDWR, not O_WRONLY: mCoreSaveStateNamed reads back through the VFile while
 * it writes, and returns false on a write-only one. */
int frlg_state_save_file(struct FrlgCore* h, const char* path) {
	struct VFile* vf = VFileOpen(path, O_RDWR | O_CREAT | O_TRUNC);
	if (!vf) {
		return 0;
	}
	int ok = mCoreSaveStateNamed(h->core, vf, SAVESTATE_SAVEDATA | SAVESTATE_RTC)
	             ? 1
	             : 0;
	vf->close(vf);
	return ok;
}

int frlg_state_load_file(struct FrlgCore* h, const char* path) {
	struct VFile* vf = VFileOpen(path, O_RDONLY);
	if (!vf) {
		return 0;
	}
	int ok = mCoreLoadStateNamed(h->core, vf, SAVESTATE_SAVEDATA | SAVESTATE_RTC)
	             ? 1
	             : 0;
	vf->close(vf);
	return ok;
}

/* --- video -------------------------------------------------------------- */

unsigned frlg_width(const struct FrlgCore* h) {
	return h->width;
}

unsigned frlg_height(const struct FrlgCore* h) {
	return h->height;
}

/* mColor is uint32_t here (flags.h leaves COLOR_16_BIT undefined), laid out
 * red in bits 0-7, green 8-15, blue 16-23, alpha 24-31 -- i.e. R,G,B,A byte
 * order in memory on a little-endian host, which is PNG's RGBA8 directly. */
const uint32_t* frlg_video_buffer(const struct FrlgCore* h) {
	return (const uint32_t*) h->video;
}

/* --- rom header --------------------------------------------------------- */

/* mGBA 0.11 replaced getGameTitle/getGameCode with a single getGameInfo.
 * The out16/out8 shapes and the "AGB-BPRE" code format are kept, so the Rust
 * side did not have to change. */

void frlg_game_title(const struct FrlgCore* h, char* out16) {
	struct mGameInfo info;
	h->core->getGameInfo(h->core, &info);
	memset(out16, 0, 16);
	memcpy(out16, info.title, 15);
}

void frlg_game_code(const struct FrlgCore* h, char* out8) {
	struct mGameInfo info;
	h->core->getGameInfo(h->core, &info);
	memset(out8, 0, 8);
	/* "AGB-BPRE" is exactly 8 bytes, so like 0.10's getGameCode the buffer is
	 * full and unterminated; the Rust side caps at the slice length. */
	size_t n = strnlen(info.system, 3);
	memcpy(out8, info.system, n);
	out8[n] = '-';
	memcpy(out8 + n + 1, info.code, strnlen(info.code, 8 - (n + 1)));
}

size_t frlg_rom_size(const struct FrlgCore* h) {
	return h->core->romSize(h->core);
}
