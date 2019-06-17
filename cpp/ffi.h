#ifndef META_FASTDL_FFI_H
#define META_FASTDL_FFI_H

#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <new>

extern "C" {

void fastdl_deinit();

void fastdl_init(const char *config_file_dir,
                 const char *gamedir,
                 char *out_url,
                 unsigned int out_size);

void fastdl_insert_to_whitelist(const char *prefix, const char *path);

} // extern "C"

#endif // META_FASTDL_FFI_H
