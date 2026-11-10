#ifndef PORTABLE_TERMINAL_PROFILE_HOST_H
#define PORTABLE_TERMINAL_PROFILE_HOST_H
/* Optional hosted C adapter. Call on a worker, never a UI or firmware thread.
 * Reads known per-user settings only; does not execute shells or write files. */
#include "portable_terminal_profiles.h"
#ifdef __cplusplus
extern "C" {
#endif
typedef int (*ps_profile_emit)(void *,const ps_terminal_profile *);
typedef void (*ps_profile_diagnostic)(void *,const char *path,const char *message);
typedef struct { unsigned files_read,profiles_emitted,errors,cancelled; } ps_profile_scan_result;
/* Records borrow storage only for the duration of emit; copy needed fields.
 * emit returning zero cancels further delivery. Every source is fully parsed
 * before any record from that source is emitted. Missing files are not errors. */
ps_profile_scan_result ps_terminal_profiles_import_file(const char *utf8_path,
    unsigned format,unsigned platform,ps_profile_emit emit,void *context,
    ps_profile_diagnostic diagnostic,void *diagnostic_context);
/* Known Windows Terminal stable/preview/unpackaged settings, VS Code stable/
 * Insiders/VSCodium user settings on Windows/Linux/macOS. Does not scan disks,
 * registry, projects, shell rc files, remote hosts or installed executables. */
ps_profile_scan_result ps_terminal_profiles_discover(ps_profile_emit emit,void *context,
    ps_profile_diagnostic diagnostic,void *diagnostic_context);
#ifdef __cplusplus
}
#endif
#endif
