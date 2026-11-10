#include "portable_terminal_profiles.h"
#include "portable_terminal.h"

enum { J_OBJECT=1,J_ARRAY,J_STRING,J_NUMBER,J_TRUE,J_FALSE,J_NULL };
typedef struct { const char *s; size_t n,at; ps_profile_workspace *w; } reader;
static int fail(reader *r,const char *message) {
    if(!r->w->error){r->w->error=message;r->w->error_at=r->at;}return 0;
}
static int put(reader *r,unsigned c) {
    if(r->w->string_used>=r->w->string_capacity)return fail(r,"String storage exhausted");
    r->w->strings[r->w->string_used++]=(char)c;return 1;
}
static const char *copy(reader *r,const char *s) {
    size_t start=r->w->string_used;
    while(*s)if(!put(r,(unsigned char)*s++))return 0;
    if(!put(r,0))return 0;
    return r->w->strings+start;
}
static int space(reader *r) {
    for(;;){
        while(r->at<r->n&&(r->s[r->at]==' '||r->s[r->at]=='\t'||r->s[r->at]=='\r'||r->s[r->at]=='\n'))++r->at;
        if(r->at+1>=r->n||r->s[r->at]!='/')return 1;
        if(r->s[r->at+1]=='/'){
            r->at+=2;while(r->at<r->n&&r->s[r->at]!='\n')++r->at;
        }else if(r->s[r->at+1]=='*'){
            r->at+=2;while(r->at+1<r->n&&(r->s[r->at]!='*'||r->s[r->at+1]!='/'))++r->at;
            if(r->at+1>=r->n)return fail(r,"Unclosed comment");
            r->at+=2;
        }else return 1;
    }
}
static int hex4(reader *r,unsigned *value) {
    unsigned i,n=0,c;
    for(i=0;i<4;++i){
        if(r->at==r->n)return fail(r,"Incomplete Unicode escape");
        c=(unsigned char)r->s[r->at++];
        if(c>='0'&&c<='9')c-='0';else if(c>='a'&&c<='f')c=c-'a'+10;
        else if(c>='A'&&c<='F')c=c-'A'+10;else return fail(r,"Invalid Unicode escape");
        n=n*16+c;
    }*value=n;return 1;
}
static int utf8(reader *r,unsigned c) {
    if(!c)return fail(r,"NUL is not permitted in profile strings");
    if(c<128)return put(r,c);
    if(c<2048)return put(r,192|(c>>6))&&put(r,128|(c&63));
    if(c<65536)return put(r,224|(c>>12))&&put(r,128|((c>>6)&63))&&put(r,128|(c&63));
    return put(r,240|(c>>18))&&put(r,128|((c>>12)&63))&&put(r,128|((c>>6)&63))&&put(r,128|(c&63));
}
static const char *string(reader *r) {
    size_t start=r->w->string_used;unsigned c,low,k,min;
    if(r->at>=r->n||r->s[r->at++]!='"'){fail(r,"Expected string");return 0;}
    while(r->at<r->n){
        c=(unsigned char)r->s[r->at++];
        if(c=='"'){if(!put(r,0))return 0;return r->w->strings+start;}
        if(c<32){fail(r,"Control character in string");return 0;}
        if(c=='\\'){
            if(r->at==r->n){fail(r,"Incomplete string escape");return 0;}
            c=(unsigned char)r->s[r->at++];
            if(c=='b')c=8;else if(c=='f')c=12;else if(c=='n')c=10;else if(c=='r')c=13;else if(c=='t')c=9;
            else if(c=='u'){
                if(!hex4(r,&c))return 0;
                if(c>=0xd800&&c<=0xdbff){
                    if(r->at+2>r->n||r->s[r->at]!='\\'||r->s[r->at+1]!='u'){fail(r,"Missing Unicode low surrogate");return 0;}
                    r->at+=2;if(!hex4(r,&low))return 0;
                    if(low<0xdc00||low>0xdfff){fail(r,"Invalid Unicode low surrogate");return 0;}
                    c=0x10000+((c-0xd800)<<10)+(low-0xdc00);
                }else if(c>=0xdc00&&c<=0xdfff){fail(r,"Unpaired Unicode surrogate");return 0;}
            }else if(c!='"'&&c!='\\'&&c!='/'){fail(r,"Invalid string escape");return 0;}
        }else if(c>=128){
            if(c>=194&&c<=223){k=1;min=128;c&=31;}
            else if(c>=224&&c<=239){k=2;min=2048;c&=15;}
            else if(c>=240&&c<=244){k=3;min=65536;c&=7;}
            else {fail(r,"Invalid UTF-8");return 0;}
            while(k--){
                if(r->at==r->n){fail(r,"Incomplete UTF-8");return 0;}
                low=(unsigned char)r->s[r->at++];
                if((low&192)!=128){fail(r,"Invalid UTF-8 continuation");return 0;}c=(c<<6)|(low&63);
            }
            if(c<min||c>0x10ffff||(c>=0xd800&&c<=0xdfff)){fail(r,"Invalid UTF-8 scalar");return 0;}
        }
        if(!utf8(r,c))return 0;
    }fail(r,"Unclosed string");return 0;
}
static unsigned value(reader *r,unsigned depth) {
    unsigned id,kind,child,last=0,members=0;const char *key;size_t start;char c;
    if(depth>32){fail(r,"JSON nesting exceeds 32 levels");return 0;}
    if(!space(r)||r->at==r->n){fail(r,"Missing JSON value");return 0;}
    if(r->w->token_count+1>=r->w->token_capacity){fail(r,"Token storage exhausted");return 0;}
    id=(unsigned)++r->w->token_count;ps_secure_clear(&r->w->tokens[id],sizeof r->w->tokens[id]);
    c=r->s[r->at];kind=c=='{'?J_OBJECT:c=='['?J_ARRAY:c=='"'?J_STRING:0;r->w->tokens[id].kind=kind;
    if(kind==J_STRING){r->w->tokens[id].text=string(r);return r->w->tokens[id].text?id:0;}
    if(kind==J_OBJECT||kind==J_ARRAY){
        char end=kind==J_OBJECT?'}':']';++r->at;if(!space(r))return 0;
        if(r->at<r->n&&r->s[r->at]==end){++r->at;return id;}
        for(;;){
            if(++members>2048){fail(r,"Container exceeds 2048 entries");return 0;}
            key=0;
            if(kind==J_OBJECT){
                unsigned previous;
                key=string(r);if(!key||!space(r))return 0;
                for(previous=r->w->tokens[id].first;previous;previous=r->w->tokens[previous].next)
                    if(ps_text_equal(key,r->w->tokens[previous].key)){fail(r,"Duplicate object key");return 0;}
                if(r->at==r->n||r->s[r->at++]!=':'){fail(r,"Expected colon");return 0;}
            }
            child=value(r,depth+1);if(!child)return 0;r->w->tokens[child].key=key;
            if(last)r->w->tokens[last].next=child;else r->w->tokens[id].first=child;last=child;
            if(!space(r)||r->at==r->n){fail(r,"Unclosed container");return 0;}
            c=r->s[r->at++];if(c==end)return id;
            if(c!=','){fail(r,"Expected comma");return 0;}
            if(!space(r))return 0;
            if(r->at<r->n&&r->s[r->at]==end){++r->at;return id;}
        }
    }
    start=r->at;
    if(c=='t'||c=='f'||c=='n'){
        const char *literal=c=='t'?"true":c=='f'?"false":"null";
        while(*literal)if(r->at==r->n||r->s[r->at++]!=*literal++){fail(r,"Invalid JSON literal");return 0;}
        r->w->tokens[id].kind=c=='t'?J_TRUE:c=='f'?J_FALSE:J_NULL;return id;
    }
    if(c=='-')++r->at;
    if(r->at==r->n){fail(r,"Invalid number");return 0;}
    if(r->s[r->at]=='0')++r->at;
    else {size_t digits=r->at;while(r->at<r->n&&r->s[r->at]>='0'&&r->s[r->at]<='9')++r->at;
        if(digits==r->at){fail(r,"Invalid JSON value");return 0;}}
    if(r->at<r->n&&r->s[r->at]=='.'){
        size_t digits=++r->at;while(r->at<r->n&&r->s[r->at]>='0'&&r->s[r->at]<='9')++r->at;
        if(digits==r->at){fail(r,"Invalid fraction");return 0;}
    }
    if(r->at<r->n&&(r->s[r->at]=='e'||r->s[r->at]=='E')){
        size_t digits;++r->at;if(r->at<r->n&&(r->s[r->at]=='+'||r->s[r->at]=='-'))++r->at;
        digits=r->at;while(r->at<r->n&&r->s[r->at]>='0'&&r->s[r->at]<='9')++r->at;
        if(digits==r->at){fail(r,"Invalid exponent");return 0;}
    }
    if(r->at==start){fail(r,"Invalid number");return 0;}r->w->tokens[id].kind=J_NUMBER;return id;
}
static unsigned get(reader *r,unsigned object,const char *key) {
    unsigned i;if(!object)return 0;
    for(i=r->w->tokens[object].first;i;i=r->w->tokens[i].next)
        if(r->w->tokens[i].key&&ps_text_equal(r->w->tokens[i].key,key))return i;
    return 0;
}
static unsigned inherited(reader *r,unsigned object,unsigned defaults,const char *key) {
    unsigned i=get(r,object,key);return i?i:get(r,defaults,key);
}
static const char *text(reader *r,unsigned id) {
    if(!id||r->w->tokens[id].kind==J_NULL)return 0;
    if(r->w->tokens[id].kind!=J_STRING){fail(r,"Expected string or null setting");return 0;}
    return r->w->tokens[id].text;
}
static int flag(reader *r,unsigned id,unsigned mask,unsigned *flags) {
    if(!id)return 1;
    if(r->w->tokens[id].kind==J_TRUE){*flags|=mask;return 1;}
    if(r->w->tokens[id].kind==J_FALSE){*flags&=~mask;return 1;}
    return fail(r,"Expected boolean setting");
}
static int unresolved(const char *s) {
    if(!s)return 0;
    if(*s=='~')return 1;
    while(*s){if(*s=='$'||*s=='%')return 1;++s;}return 0;
}
static int env_equal(const char *a,const char *b,unsigned platform) {
    if(platform!=PS_PROFILE_WINDOWS)return ps_text_equal(a,b);
    while(*a&&ps_ascii_fold((unsigned char)*a)==ps_ascii_fold((unsigned char)*b)){++a;++b;}return *a==*b;
}
static int environment(reader *r,unsigned id,ps_terminal_profile *p) {
    unsigned i,j;const char *s;
    if(!id)return 1;
    if(r->w->tokens[id].kind==J_NULL){p->environment_count=0;return 1;}
    if(r->w->tokens[id].kind!=J_OBJECT)return fail(r,"Environment must be an object");
    for(i=r->w->tokens[id].first;i;i=r->w->tokens[i].next){
        s=r->w->tokens[i].key;if(!*s)return fail(r,"Empty environment key");
        while(*s)if(*s++=='=')return fail(r,"Invalid environment key");
        for(j=0;j<p->environment_count;++j)if(env_equal(p->environment[j].name,r->w->tokens[i].key,p->platform))break;
        if(j==PS_PROFILE_ENVIRONMENT)return fail(r,"Too many environment variables");
        if(j==p->environment_count)++p->environment_count;
        p->environment[j].name=r->w->tokens[i].key;p->environment[j].value=text(r,i);
        p->environment[j].unset=r->w->tokens[i].kind==J_NULL;
        if(r->w->error)return 0;
        if(unresolved(p->environment[j].value))p->flags|=PS_PROFILE_UNRESOLVED;
    }return 1;
}
/* Windows CRT quoting only, never cmd/PowerShell evaluation or expansion.
 * The executable token has the documented special first-argument rules. */
static int command(reader *r,const char *line,ps_terminal_profile *p,int program) {
    size_t i=0,start;unsigned quote,slashes,k;const char *word;
    while(line[i]){
        while(line[i]==' '||line[i]=='\t')++i;
        if(!line[i])break;
        start=r->w->string_used;quote=0;
        while(line[i]&&(quote||(line[i]!=' '&&line[i]!='\t'))){
            if(line[i]=='\r'||line[i]=='\n')return fail(r,"Multiline command line is not supported");
            if(program){if(line[i]=='"'){quote=!quote;++i;}else if(!put(r,(unsigned char)line[i++]))return 0;continue;}
            slashes=0;while(line[i]=='\\'){++slashes;++i;}
            if(line[i]=='"'){
                for(k=0;k<slashes/2;++k)if(!put(r,'\\'))return 0;
                if(slashes%2){if(!put(r,'"'))return 0;++i;}
                else if(quote&&line[i+1]=='"'){if(!put(r,'"'))return 0;i+=2;}
                else {quote=!quote;++i;}
            }else {
                for(k=0;k<slashes;++k)if(!put(r,'\\'))return 0;
                if(line[i]&&(quote||(line[i]!=' '&&line[i]!='\t')))if(!put(r,(unsigned char)line[i++]))return 0;
            }
        }
        if(quote)return fail(r,"Unclosed command-line quote");
        if(!put(r,0))return 0;
        word=r->w->strings+start;
        if(program){if(!*word)return fail(r,"Empty executable");p->executable=word;program=0;}
        else {if(p->argument_count==PS_PROFILE_ARGUMENTS)return fail(r,"Too many command arguments");p->arguments[p->argument_count++]=word;}
    }return 1;
}
static int args(reader *r,unsigned id,ps_terminal_profile *p) {
    unsigned i;if(!id)return 1;
    if(r->w->tokens[id].kind==J_STRING&&p->platform==PS_PROFILE_WINDOWS)return command(r,r->w->tokens[id].text,p,0);
    if(r->w->tokens[id].kind!=J_ARRAY)return fail(r,"Arguments must be an array (or Windows command-line string)");
    for(i=r->w->tokens[id].first;i;i=r->w->tokens[i].next){
        if(p->argument_count==PS_PROFILE_ARGUMENTS)return fail(r,"Too many command arguments");
        if(r->w->tokens[i].kind!=J_STRING)return fail(r,"Arguments must contain strings");
        p->arguments[p->argument_count++]=text(r,i);
    }return 1;
}
static int identity(reader *r,ps_terminal_profile *p,const char *identity) {
    char prefix[48];size_t start=r->w->string_used;const char *s;
    ps_terminal_format(prefix,sizeof prefix,"%u:%u:%u:",p->format,p->platform,(unsigned)ps_text_length(p->provenance));
    for(s=prefix;*s;++s)if(!put(r,(unsigned char)*s))return 0;
    for(s=p->provenance;*s;++s)if(!put(r,(unsigned char)*s))return 0;
    if(!put(r,':'))return 0;
    for(s=identity;*s;++s)if(!put(r,(unsigned char)*s))return 0;
    if(!put(r,0))return 0;
    p->stable_id=r->w->strings+start;return 1;
}
static int normalize(reader *r,unsigned object,unsigned defaults,ps_terminal_profile *p) {
    unsigned id,i;const char *key=p->format==PS_PROFILE_LIST_DEFAULTS?"startingDirectory":"cwd";
    if(r->w->tokens[object].kind==J_NULL){p->flags|=PS_PROFILE_DISABLED;return 1;}
    if(r->w->tokens[object].kind!=J_OBJECT)return fail(r,"Profile must be an object or disabled null entry");
    p->source=text(r,inherited(r,object,defaults,"source"));
    p->icon=text(r,inherited(r,object,defaults,"icon"));
    id=inherited(r,object,defaults,key);p->cwd=text(r,id);
    if(id&&r->w->tokens[id].kind==J_NULL)p->flags|=PS_PROFILE_CWD_INHERIT;
    if(!id&&p->format==PS_PROFILE_LIST_DEFAULTS)p->cwd="%USERPROFILE%";
    if(!flag(r,inherited(r,object,defaults,"hidden"),PS_PROFILE_HIDDEN,&p->flags)
       ||!flag(r,inherited(r,object,defaults,"disabled"),PS_PROFILE_DISABLED,&p->flags)
       ||!flag(r,inherited(r,object,defaults,"elevate"),PS_PROFILE_REQUIRES_ELEVATION,&p->flags))return 0;
    if(p->flags&PS_PROFILE_REQUIRES_ELEVATION)p->flags|=PS_PROFILE_UNRESOLVED;
    if(!environment(r,get(r,defaults,"environment"),p)||!environment(r,get(r,defaults,"env"),p)
       ||!environment(r,get(r,object,"environment"),p)||!environment(r,get(r,object,"env"),p))return 0;
    if(p->format==PS_PROFILE_LIST_DEFAULTS){
        p->command_line=text(r,inherited(r,object,defaults,"commandline"));
        if(p->command_line){if(!command(r,p->command_line,p,1))return 0;}
        else if(!p->source)p->executable="cmd.exe";
    }else {
        id=get(r,object,"path");
        if(id&&r->w->tokens[id].kind==J_ARRAY){
            for(i=r->w->tokens[id].first;i;i=r->w->tokens[i].next){
                if(p->path_count==PS_PROFILE_PATHS)return fail(r,"Too many executable alternatives");
                if(r->w->tokens[i].kind!=J_STRING||!*text(r,i))return fail(r,"Executable alternatives must be nonempty strings");
                p->path_candidates[p->path_count++]=text(r,i);
            }
            if(!p->path_count)return fail(r,"Executable alternatives are empty");
            p->flags|=PS_PROFILE_UNRESOLVED;
        }else p->executable=text(r,id);
        if(!args(r,get(r,object,"args"),p))return 0;
    }
    if(!p->executable||!*p->executable||unresolved(p->executable)||unresolved(p->cwd))p->flags|=PS_PROFILE_UNRESOLVED;
    for(i=0;i<p->argument_count;++i)if(unresolved(p->arguments[i]))p->flags|=PS_PROFILE_UNRESOLVED;
    return !r->w->error;
}
int ps_terminal_profiles_parse(const char *json,size_t length,unsigned format,unsigned platform,
    const char *provenance,ps_profile_workspace *w,ps_terminal_profile *profiles,size_t capacity,size_t *count) {
    reader r;unsigned root,container,defaults=0,i,j,global_env=0,global_cwd=0,inherit_env=0,disabled_sources=0;
    size_t n=0,k;const char *selected,*identifier,*origin;char map_key[64],default_key[64],env_key[64];
    if(count)*count=0;
    if(!w)return 0;
    w->error=0;w->error_at=w->token_count=w->string_used=0;r.s=json;r.n=length;r.at=0;r.w=w;
    if(!count||!json||!provenance||!profiles||!capacity||!w->tokens||w->token_capacity<2
       ||w->token_capacity>PS_PROFILE_MAX_TOKENS||!w->strings||!w->string_capacity
       ||length>PS_PROFILE_MAX_INPUT||ps_text_length(provenance)>4096
       ||(format!=PS_PROFILE_LIST_DEFAULTS&&format!=PS_PROFILE_NAMED_MAP)
       ||platform<PS_PROFILE_WINDOWS||platform>PS_PROFILE_MACOS)return fail(&r,"Invalid import bounds or format");
    if(length>=3&&(unsigned char)json[0]==239&&(unsigned char)json[1]==187&&(unsigned char)json[2]==191)r.at=3;
    root=value(&r,0);if(!root||!space(&r))return 0;
    if(r.at!=length||w->tokens[root].kind!=J_OBJECT)return fail(&r,"Expected one complete settings object");
    origin=copy(&r,provenance);if(!origin)return 0;
    if(format==PS_PROFILE_LIST_DEFAULTS){
        container=get(&r,root,"profiles");selected=text(&r,get(&r,root,"defaultProfile"));
        disabled_sources=get(&r,root,"disabledProfileSources");
        if(disabled_sources&&w->tokens[disabled_sources].kind!=J_ARRAY)return fail(&r,"Disabled profile sources must be an array");
        if(container&&w->tokens[container].kind==J_OBJECT){defaults=get(&r,container,"defaults");container=get(&r,container,"list");}
        if(!container||w->tokens[container].kind!=J_ARRAY)return fail(&r,"Profile list is missing or invalid");
        if(defaults&&w->tokens[defaults].kind!=J_OBJECT)return fail(&r,"Profile defaults must be an object");
    }else {
        const char *suffix=platform==PS_PROFILE_WINDOWS?"windows":platform==PS_PROFILE_LINUX?"linux":"osx";
        ps_terminal_format(map_key,sizeof map_key,"terminal.integrated.profiles.%s",suffix);
        ps_terminal_format(default_key,sizeof default_key,"terminal.integrated.defaultProfile.%s",suffix);
        ps_terminal_format(env_key,sizeof env_key,"terminal.integrated.env.%s",suffix);
        container=get(&r,root,map_key);selected=text(&r,get(&r,root,default_key));global_env=get(&r,root,env_key);
        global_cwd=get(&r,root,"terminal.integrated.cwd");inherit_env=get(&r,root,"terminal.integrated.inheritEnv");
        if(inherit_env&&w->tokens[inherit_env].kind!=J_TRUE&&w->tokens[inherit_env].kind!=J_FALSE)return fail(&r,"inheritEnv must be boolean");
        if(!container||w->tokens[container].kind!=J_OBJECT)return fail(&r,"Named profile map is missing or invalid");
    }
    if(w->error)return 0;
    for(i=w->tokens[container].first;i;i=w->tokens[i].next){
        ps_terminal_profile *p;
        if(n==capacity)return fail(&r,"Profile output capacity exhausted");
        p=&profiles[n];ps_secure_clear(p,sizeof *p);
        p->format=format;p->platform=platform;p->provenance=origin;
        if(format==PS_PROFILE_LIST_DEFAULTS){
            if(w->tokens[i].kind!=J_OBJECT)return fail(&r,"List profile must be an object");
            p->name=text(&r,get(&r,i,"name"));identifier=text(&r,get(&r,i,"guid"));
            if(identifier){
                size_t start=w->string_used;const char *c;
                for(c=identifier;*c;++c)if(!put(&r,ps_ascii_fold((unsigned char)*c)))return 0;
                if(!put(&r,0))return 0;
                identifier=w->strings+start;
            }
            if(!p->name)p->name=identifier;
            if(!identifier)identifier=p->name;
        }else identifier=p->name=w->tokens[i].key;
        if(!p->name||!*p->name||!identifier||!*identifier||w->error)return fail(&r,"Profile requires a name or identifier");
        if(!environment(&r,global_env,p)||!normalize(&r,i,defaults,p)||!identity(&r,p,identifier))return 0;
        if(global_cwd&&!p->cwd&&!(p->flags&PS_PROFILE_CWD_INHERIT)){
            p->cwd=text(&r,global_cwd);
            if(w->error)return 0;
            if(!p->cwd)p->flags|=PS_PROFILE_CWD_INHERIT;
            if(unresolved(p->cwd))p->flags|=PS_PROFILE_UNRESOLVED;
        }
        if(inherit_env&&w->tokens[inherit_env].kind==J_FALSE)p->flags|=PS_PROFILE_ENV_CLEAR;
        if(selected&&((format==PS_PROFILE_LIST_DEFAULTS&&get(&r,i,"guid")?env_equal(selected,identifier,PS_PROFILE_WINDOWS):ps_text_equal(selected,identifier))||ps_text_equal(selected,p->name)))p->flags|=PS_PROFILE_DEFAULT;
        if(disabled_sources)for(j=w->tokens[disabled_sources].first;j;j=w->tokens[j].next){
            const char *source=text(&r,j);if(!source)return fail(&r,"Disabled source names must be strings");
            if(p->source&&ps_text_equal(source,p->source))p->flags|=PS_PROFILE_DISABLED;
        }
        for(k=0;k<n;++k)if(ps_text_equal(p->stable_id,profiles[k].stable_id))return fail(&r,"Duplicate profile identity");
        ++n;
    }
    if(w->error)return 0;
    *count=n;return 1;
}
