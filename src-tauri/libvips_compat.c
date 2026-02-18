/*
 * libvips_compat.c
 *
 * Compatibility shims for API changes between the libvips version that
 * libvips-rs 8.15.1 was generated against and the libvips 8.18 Windows
 * binaries from build-win64-mxe.
 *
 * These stubs are compiled into a static archive and linked BEFORE libvips,
 * so the linker satisfies the symbol references from libvips_rs without
 * needing the old names to exist in the DLL.
 */

typedef struct _VipsTarget VipsTarget;
typedef struct _VipsImage  VipsImage;

/* Declare the new name so we can forward to it at runtime. */
extern void vips_target_end(VipsTarget *target);

/*
 * vips_target_finish -> vips_target_end
 * Renamed in libvips 8.17.  Forward the call to the current symbol.
 */
void vips_target_finish(VipsTarget *target) {
    vips_target_end(target);
}

/*
 * vips_rawsave_fd was removed in libvips 8.17+.
 * We never call this function in the app's image processing pipeline.
 * The stub always returns an error code.
 */
int vips_rawsave_fd(VipsImage *in, int fd, ...) {
    (void)in; (void)fd;
    return -1;
}

/*
 * vips_cache (the "cache" operation shorthand) was restructured in 8.17+.
 * We never call this function in the app's image processing pipeline.
 * The stub always returns an error code.
 */
int vips_cache(VipsImage *in, VipsImage **out, ...) {
    (void)in; (void)out;
    return -1;
}
