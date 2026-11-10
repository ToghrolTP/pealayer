/* Pealayer-owned adapter. The upstream registry/editor remains unmodified.
   All callbacks are byte/state callbacks; no OS, process or application calls. */
#include "portable_settings_editor.h"
#include <stddef.h>
#include <string.h>
#define PH_SETTINGS 64
#define PH_TEXT 192
typedef int (*ph_write_fn)(void *, const char *, const char *, const char *);
typedef struct ph_editor ph_editor;
typedef struct { ph_editor *owner; unsigned index; char name[96], value[PH_TEXT]; int pending; } ph_row;
struct ph_editor {
    ps_settings registry;
    const ps_setting *entries[PH_SETTINGS];
    ps_setting definitions[PH_SETTINGS];
    ph_row values[PH_SETTINGS];
    ps_settings_editor editor;
    ps_settings_editor_row rows[PH_SETTINGS];
    ps_control controls[PH_SETTINGS];
    ph_write_fn write;
    void *context;
};
unsigned ph_editor_size(void) { return sizeof(ph_editor); }
unsigned ph_editor_align(void) { struct a { char c; ph_editor e; }; return offsetof(struct a,e); }
void ph_editor_init(ph_editor *e, ph_write_fn write, void *context) {
    memset(e,0,sizeof *e); e->write=write; e->context=context;
    ps_settings_init(&e->registry,e->entries,PH_SETTINGS);
}
static int read_value(void *context,ps_setting_value *value) {
    ph_row *r=(ph_row *)context;
    return ps_setting_parse(&r->owner->definitions[r->index],r->value,value)==PS_SETTING_OK;
}
static int write_value(void *context,const ps_setting_value *value) {
    ph_row *r=(ph_row *)context;
    if(r->pending || !r->owner->write(r->owner->context,r->name,value->text,r->value))return 0;
    r->pending=1; return 1;
}
static int panel_read(void *context,unsigned id,ps_control_value *out) {
    ph_editor *e=(ph_editor *)context;
    int ok=ps_settings_editor_read(&e->editor,id,out);
    if(id<e->registry.count && e->values[id].pending)out->flags|=PS_CONTROL_PENDING;
    return ok;
}
static int panel_write(void *context,unsigned id,const ps_control_value *value,ps_u32 revision) {
    ph_editor *e=(ph_editor *)context;
    return ps_settings_editor_write(&e->editor,id,value,revision)?2:0;
}
#if PS_PANEL_TEXT
static unsigned panel_text(void *context,unsigned id,char *text,ps_byte *roles,unsigned capacity) {
    ph_editor *e=(ph_editor *)context;return ps_settings_editor_text(&e->editor,id,text,roles,capacity);
}
#endif
int ph_editor_value(ph_editor *e,const char *name,const char *value,unsigned kind) {
    unsigned i=e->registry.count;
    if(strlen(name)>=sizeof e->values[0].name || strlen(value)>=PH_TEXT)return 0;
    for(unsigned j=0;j<i;++j)if(!strcmp(e->values[j].name,name)){i=j;break;}
    if(i>=PH_SETTINGS)return 0;
    ph_row *r=&e->values[i];
    if(i==e->registry.count) {
        r->owner=e;r->index=i;strcpy(r->name,name);
        ps_setting *d=&e->definitions[i];d->name=r->name;d->type=kind;
        d->minimum=100;d->maximum=10000;
        d->max_length=PH_TEXT-1;d->read=read_value;d->write=write_value;d->context=r;
        if(ps_settings_register(&e->registry,d))return 0;
    }
    strcpy(r->value,value);r->pending=0;return 1;
}
int ph_editor_open(ph_editor *e,ps_session *s) {
    static const ps_column columns[]={{"Setting",28,48,1},{"Value",16,0,2}};
    if(!ps_settings_editor_init(&e->editor,&e->registry,e->controls,e->rows,PH_SETTINGS,"Pealayer settings"))return 0;
    /* The host supplies required panel metadata; no persistent filler notice. */
    e->editor.panel.status="";
    e->editor.chrome.columns=columns;e->editor.chrome.count=2;
    e->editor.panel.read=panel_read;e->editor.panel.write=panel_write;e->editor.panel.context=e;
#if PS_PANEL_TEXT
    e->editor.chrome.text=panel_text;
#endif
    return ps_settings_editor_open(&e->editor,s);
}
void ph_help_setup(ps_session *s) {
    static const ps_help_item set_args[]={{"KEY","Dotted key from get; Tab completes names"},{"VALUE","Validated value; quote strings containing spaces"}};
    static const ps_help_item examples[]={{"configure","Edit settings in-place; Enter applies, Esc cancels"},{"get ui.terminal.font_size","Read the current host value"},{"set ui.terminal.font_size 14","Apply and save through the same owner as Preferences"}};
    static const ps_help_document docs[]={
        {"configure","Live host configuration. Changes from other surfaces refresh this table. A pending edit is not an applied value.",0,0,0,0,examples,3},
        {"get","Inspect the host's curated, non-secret settings. Security policy and private paths are not exposed.",set_args,1,0,0,examples+1,1},
        {"set","Validate, apply and save one setting. Stale inline edits are rejected, not overwritten. Shell commands cannot grant themselves execution rights.",set_args,2,0,0,examples+2,1},
        {"help","Command help. Tab completes commands, F7 opens history, Shift+Tab reverses completion, Ctrl-L redraws, Ctrl-C cancels input.",0,0,0,0,0,0}
    };
    ps_set_help_documents(s,docs,sizeof docs/sizeof docs[0]);
}
