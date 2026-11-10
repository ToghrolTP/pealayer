#include "portable_terminal_profile_host.h"
#include "portable_terminal.h"
#include <stdlib.h>
#include <stdio.h>
#include <errno.h>
#ifdef _WIN32
#include <windows.h>
#endif

static void report(ps_profile_scan_result *r,ps_profile_diagnostic diagnostic,void *ctx,
                   const char *path,const char *message) {
    ++r->errors;if(diagnostic)diagnostic(ctx,path,message);
}
/* 0 missing, -1 inaccessible/invalid, 1 complete read. */
static int read_file(const char *path,char *data,size_t *length) {
#ifdef _WIN32
    wchar_t wide[32768];HANDLE file;LARGE_INTEGER size;DWORD done,extra;char byte;
    if(!MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,path,-1,wide,32768))return -1;
    file=CreateFileW(wide,GENERIC_READ,FILE_SHARE_READ|FILE_SHARE_WRITE|FILE_SHARE_DELETE,0,OPEN_EXISTING,FILE_ATTRIBUTE_NORMAL,0);
    if(file==INVALID_HANDLE_VALUE){DWORD error=GetLastError();return error==ERROR_FILE_NOT_FOUND||error==ERROR_PATH_NOT_FOUND?0:-1;}
    if(!GetFileSizeEx(file,&size)||size.QuadPart<0||size.QuadPart>PS_PROFILE_MAX_INPUT){CloseHandle(file);return -1;}
    if(!ReadFile(file,data,(DWORD)size.QuadPart,&done,0)||done!=(DWORD)size.QuadPart
       ||!ReadFile(file,&byte,1,&extra,0)||extra){CloseHandle(file);return -1;}
    CloseHandle(file);*length=done;return 1;
#else
    FILE *file=fopen(path,"rb");size_t n;
    if(!file)return errno==ENOENT||errno==ENOTDIR?0:-1;
    n=fread(data,1,PS_PROFILE_MAX_INPUT+1,file);
    if(ferror(file)||n>PS_PROFILE_MAX_INPUT){fclose(file);return -1;}
    fclose(file);*length=n;return 1;
#endif
}
ps_profile_scan_result ps_terminal_profiles_import_file(const char *path,unsigned format,
    unsigned platform,ps_profile_emit emit,void *ctx,ps_profile_diagnostic diagnostic,void *diagnostic_ctx) {
    ps_profile_scan_result result={0,0,0,0};ps_profile_workspace w;size_t length=0,count=0,i;
    char *data=0;ps_terminal_profile *profiles=0;int read;
    ps_secure_clear(&w,sizeof w);
    if(!path||!emit){report(&result,diagnostic,diagnostic_ctx,path?path:"","Path and callback are required");return result;}
    data=(char *)malloc(PS_PROFILE_MAX_INPUT+1);
    if(!data){report(&result,diagnostic,diagnostic_ctx,path,"Profile read allocation failed");return result;}
    read=read_file(path,data,&length);
    if(read==0){free(data);return result;}
    if(read<0){report(&result,diagnostic,diagnostic_ctx,path,"Settings could not be read completely within 1 MiB");free(data);return result;}
    ++result.files_read;
    w.token_capacity=PS_PROFILE_MAX_TOKENS;w.string_capacity=4*PS_PROFILE_MAX_INPUT;
    w.tokens=(ps_profile_token *)malloc(w.token_capacity*sizeof *w.tokens);
    w.strings=(char *)malloc(w.string_capacity);profiles=(ps_terminal_profile *)malloc(128*sizeof *profiles);
    if(!w.tokens||!w.strings||!profiles)report(&result,diagnostic,diagnostic_ctx,path,"Profile import allocation failed");
    else if(!ps_terminal_profiles_parse(data,length,format,platform,path,&w,profiles,128,&count))
        report(&result,diagnostic,diagnostic_ctx,path,w.error?w.error:"Profile import failed");
    else for(i=0;i<count;++i){
        if(!emit(ctx,&profiles[i])){result.cancelled=1;break;}++result.profiles_emitted;
    }
    /* Environment values may be private; no persisted cache or diagnostic dump. */
    if(w.strings)ps_secure_clear(w.strings,w.string_used);
    ps_secure_clear(data,length);free(data);free(w.tokens);free(w.strings);free(profiles);return result;
}
static int environment_path(const char *name,char *out,size_t capacity) {
#ifdef _WIN32
    wchar_t key[64],value[32768];DWORD length;
    if(!MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,name,-1,key,64))return 0;
    length=GetEnvironmentVariableW(key,value,32768);
    if(!length||length>=32768)return 0;
    return WideCharToMultiByte(CP_UTF8,WC_ERR_INVALID_CHARS,value,-1,out,(int)capacity,0,0)>0;
#else
    const char *value=getenv(name);size_t length;
    if(!value||!*value)return 0;
    length=ps_text_length(value);
    if(length>=capacity)return 0;
    ps_bytes_copy(out,value,length+1);return 1;
#endif
}
static void source(ps_profile_scan_result *total,const char *base,const char *suffix,
    unsigned format,unsigned platform,ps_profile_emit emit,void *ctx,ps_profile_diagnostic diagnostic,void *diagnostic_ctx) {
    char path[8192];size_t a=ps_text_length(base),b=ps_text_length(suffix);ps_profile_scan_result part;
    if(total->cancelled||!a)return;
    if(a+b>=sizeof path){report(total,diagnostic,diagnostic_ctx,base,"Settings path exceeds supported length");return;}
    ps_bytes_copy(path,base,a);ps_bytes_copy(path+a,suffix,b+1);
    part=ps_terminal_profiles_import_file(path,format,platform,emit,ctx,diagnostic,diagnostic_ctx);
    total->files_read+=part.files_read;total->profiles_emitted+=part.profiles_emitted;
    total->errors+=part.errors;total->cancelled|=part.cancelled;
}
ps_profile_scan_result ps_terminal_profiles_discover(ps_profile_emit emit,void *ctx,
    ps_profile_diagnostic diagnostic,void *diagnostic_ctx) {
    ps_profile_scan_result result={0,0,0,0};char base[8192];unsigned platform;
    if(!emit){report(&result,diagnostic,diagnostic_ctx,"","Profile callback is required");return result;}
#ifdef _WIN32
    platform=PS_PROFILE_WINDOWS;
    if(environment_path("LOCALAPPDATA",base,sizeof base)){
        source(&result,base,"/Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState/settings.json",PS_PROFILE_LIST_DEFAULTS,platform,emit,ctx,diagnostic,diagnostic_ctx);
        source(&result,base,"/Packages/Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe/LocalState/settings.json",PS_PROFILE_LIST_DEFAULTS,platform,emit,ctx,diagnostic,diagnostic_ctx);
        source(&result,base,"/Microsoft/Windows Terminal/settings.json",PS_PROFILE_LIST_DEFAULTS,platform,emit,ctx,diagnostic,diagnostic_ctx);
    }
    if(!environment_path("APPDATA",base,sizeof base))return result;
#elif defined(__APPLE__)
    platform=PS_PROFILE_MACOS;
    if(!environment_path("HOME",base,sizeof base))return result;
    {const char *suffix="/Library/Application Support";size_t a=ps_text_length(base),b=ps_text_length(suffix);
     if(a+b>=sizeof base){report(&result,diagnostic,diagnostic_ctx,base,"Settings path too long");return result;}
     ps_bytes_copy(base+a,suffix,b+1);}
#else
    platform=PS_PROFILE_LINUX;
    if(!environment_path("XDG_CONFIG_HOME",base,sizeof base)){
        size_t a;if(!environment_path("HOME",base,sizeof base))return result;a=ps_text_length(base);
        if(a+8>=sizeof base){report(&result,diagnostic,diagnostic_ctx,base,"Settings path too long");return result;}
        ps_bytes_copy(base+a,"/.config",9);
    }
#endif
    source(&result,base,"/Code/User/settings.json",PS_PROFILE_NAMED_MAP,platform,emit,ctx,diagnostic,diagnostic_ctx);
    source(&result,base,"/Code - Insiders/User/settings.json",PS_PROFILE_NAMED_MAP,platform,emit,ctx,diagnostic,diagnostic_ctx);
    source(&result,base,"/VSCodium/User/settings.json",PS_PROFILE_NAMED_MAP,platform,emit,ctx,diagnostic,diagnostic_ctx);
    return result;
}
