#ifndef PORTABLE_SEMANTIC_H
#define PORTABLE_SEMANTIC_H
/* Private, namespaced OSC extension. Ordinary terminals retain the preceding
   SGR fallback. Receivers pass a complete payload, excluding OSC/ST or BEL. */
#define PS_SEMANTIC_ACCENT_SEQUENCE "\033]777;ps;ink;accent\033\\"
enum { PS_SEMANTIC_NONE, PS_SEMANTIC_ACCENT_FOREGROUND };
static inline unsigned ps_semantic_osc_role(const char *payload,unsigned length){
    static const char accent[]="777;ps;ink;accent";unsigned i;
    if(!payload||length!=sizeof accent-1)return PS_SEMANTIC_NONE;
    for(i=0;i<length;++i)if(payload[i]!=accent[i])return PS_SEMANTIC_NONE;
    return PS_SEMANTIC_ACCENT_FOREGROUND;
}
#endif
