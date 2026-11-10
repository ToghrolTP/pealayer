#ifndef PORTABLE_TERMINAL_PROFILES_H
#define PORTABLE_TERMINAL_PROFILES_H
/* MIT. Bounded C99 profile import. No allocation, OS access, expansion or
 * execution. Strings borrow workspace storage until the next parse. */
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif
enum { PS_PROFILE_LIST_DEFAULTS=1, PS_PROFILE_NAMED_MAP=2 };
enum { PS_PROFILE_WINDOWS=1, PS_PROFILE_LINUX=2, PS_PROFILE_MACOS=3 };
enum { PS_PROFILE_HIDDEN=1, PS_PROFILE_DISABLED=2, PS_PROFILE_DEFAULT=4,
       PS_PROFILE_UNRESOLVED=8, PS_PROFILE_CWD_INHERIT=16,
       PS_PROFILE_ENV_CLEAR=32, PS_PROFILE_REQUIRES_ELEVATION=64 };
enum { PS_PROFILE_ARGUMENTS=64, PS_PROFILE_ENVIRONMENT=64, PS_PROFILE_PATHS=8,
       PS_PROFILE_MAX_INPUT=1048576, PS_PROFILE_MAX_TOKENS=16384 };
typedef struct { const char *name,*value; unsigned unset; } ps_profile_env;
typedef struct {
    const char *stable_id,*name,*provenance,*source,*command_line,*executable,*cwd,*icon;
    const char *arguments[PS_PROFILE_ARGUMENTS];
    const char *path_candidates[PS_PROFILE_PATHS];
    ps_profile_env environment[PS_PROFILE_ENVIRONMENT];
    unsigned argument_count,path_count,environment_count,flags,format,platform;
} ps_terminal_profile;
/* Workspace tokens are implementation data, exposed only for caller allocation. */
typedef struct { unsigned kind,first,next; const char *key,*text; } ps_profile_token;
typedef struct {
    ps_profile_token *tokens; size_t token_capacity;
    char *strings; size_t string_capacity;
    size_t token_count,string_used,error_at; const char *error;
} ps_profile_workspace;
/* Returns 1 on complete success, 0 on invalid/unsupported/oversized data.
 * *count is zero on failure: never consume partial records. Duplicate object
 * keys/IDs are rejected; unknown well-formed settings are ignored. JSONC permits
 * comments and trailing commas. A BOM is allowed only at the beginning.
 * Raw command_line remains available even after Windows argv normalization.
 * Arguments exclude executable. Unresolved sources/path alternatives/variable
 * references remain explicit and must be reviewed/resolved by the host. */
int ps_terminal_profiles_parse(const char *json,size_t length,unsigned format,
    unsigned platform,const char *provenance,ps_profile_workspace *workspace,
    ps_terminal_profile *profiles,size_t capacity,size_t *count);
#ifdef __cplusplus
}
#endif
#endif
