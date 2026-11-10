/* Pealayer host adapter: compose the owner's editors, never duplicate them.
   Descriptors/strings are borrowed from the Rust host until detach/rebuild. */
#include "portable_settings_editor.h"
#include <stddef.h>
#include <string.h>
#define PH_SETTINGS 256
#define PH_PAGES 32
#define PH_TEXT 192
typedef int (*ph_write_fn)(void *,const char *,const char *,const char *);
typedef void (*ph_open_fn)(void *,const char *);
typedef struct ph_editor ph_editor;
typedef struct {
    const char *key,*label,*page,*group;
    const char *const *choices,*const *choice_labels;
    unsigned choice_count,kind,read_only;
} ph_definition;
typedef struct {ph_editor *owner;unsigned index,page;char name[96],value[PH_TEXT];int pending,numeric;} ph_row;
typedef struct {
    ps_settings registry;ps_settings_editor editor;
    unsigned start,count;
} ph_page;
struct ph_editor {
    const ps_setting *entries[PH_SETTINGS];ps_setting definitions[PH_SETTINGS];
    ph_row values[PH_SETTINGS];ps_settings_editor_row rows[PH_SETTINGS];
    ps_control controls[PH_SETTINGS];ps_setting_presentation metadata[PH_SETTINGS];
    ph_page sections[PH_PAGES];ps_control_page pages[PH_PAGES];
    ps_panel panel;ps_panel_chrome chrome;ps_panel_presentation presentation;
    unsigned count,page_count;int key_names;
    ph_write_fn write;ph_open_fn open;void *context;
};
unsigned ph_editor_size(void){return sizeof(ph_editor);}
unsigned ph_editor_align(void){struct a{char c;ph_editor e;};return offsetof(struct a,e);}
void ph_editor_init(ph_editor *e,ph_write_fn write,ph_open_fn open,void *context){
    memset(e,0,sizeof *e);e->write=write;e->open=open;e->context=context;
}
static int read_value(void *context,ps_setting_value *out){
    ph_row *r=(ph_row *)context;
    return ps_setting_parse(&r->owner->definitions[r->index],r->value,out)==PS_SETTING_OK;
}
static int write_value(void *context,const ps_setting_value *value){
    ph_row *r=(ph_row *)context;
    if(r->pending||!r->owner->write(r->owner->context,r->name,value->text,r->value))return 0;
    r->pending=1;return 1;
}
int ph_editor_define(ph_editor *e,const ph_definition *source){
    unsigned i=e->count,p=e->page_count;
    if(i>=PH_SETTINGS||strlen(source->key)>=96||!source->page||source->choice_count>64)return 0;
    if(p&&strcmp(e->pages[p-1].title,source->page)==0)--p;
    else {
        if(p>=PH_PAGES)return 0;
        e->sections[p].start=i;e->pages[p].title=source->page;++e->page_count;
    }
    ph_row *r=e->values+i;r->owner=e;r->index=i;r->page=p;
    r->numeric=source->kind==5;strcpy(r->name,source->key);
    ps_setting *d=e->definitions+i;d->name=r->name;d->group=source->group;
    d->type=r->numeric?PS_SETTING_TEXT:source->kind;d->choices=source->choices;
    d->choice_count=source->choice_count;d->max_length=PH_TEXT-1;
    d->read=read_value;d->write=source->read_only?0:write_value;d->context=r;
    e->entries[i]=d;
    e->metadata[i].key=r->name;e->metadata[i].label=source->label;
    e->metadata[i].choice_labels=source->choice_labels;e->metadata[i].choice_count=source->choice_count;
    ++e->sections[p].count;++e->count;return 1;
}
int ph_editor_value(ph_editor *e,const char *name,const char *value){
    if(strlen(value)>=PH_TEXT)return 0;
    for(unsigned i=0;i<e->count;++i)if(!strcmp(e->values[i].name,name)){
        strcpy(e->values[i].value,value);e->values[i].pending=0;return 1;
    }
    return 0;
}
static ps_settings_editor *local_editor(ph_editor *e,unsigned id,unsigned *local){
    if(id>=e->count)return 0;
    ph_page *p=e->sections+e->values[id].page;*local=id-p->start;return &p->editor;
}
static int panel_read(void *context,unsigned id,ps_control_value *out){
    ph_editor *e=(ph_editor *)context;unsigned local;ps_settings_editor *editor=local_editor(e,id,&local);
    if(!editor)return 0;int ok=ps_settings_editor_read(editor,local,out);
    if(e->values[id].pending)out->flags|=PS_CONTROL_PENDING;return ok;
}
static int panel_write(void *context,unsigned id,const ps_control_value *value,ps_u32 revision){
    ph_editor *e=(ph_editor *)context;unsigned local;ps_settings_editor *editor=local_editor(e,id,&local);
    return editor&&ps_settings_editor_write(editor,local,value,revision)?2:0;
}
static const char *panel_label(void *context,unsigned id){
    ph_editor *e=(ph_editor *)context;unsigned local;ps_settings_editor *editor=local_editor(e,id,&local);
    return editor?ps_settings_editor_label(editor,local):0;
}
static const char *panel_key_name(void *context,unsigned id){
    ph_editor *e=(ph_editor *)context;return id<e->count?e->values[id].name:0;
}
static const char *panel_choice(void *context,unsigned id,unsigned choice){
    ph_editor *e=(ph_editor *)context;unsigned local;ps_settings_editor *editor=local_editor(e,id,&local);
    return editor?ps_settings_editor_choice_label(editor,local,choice):0;
}
#if PS_PANEL_TEXT
static unsigned panel_text(void *context,unsigned id,char *text,ps_byte *roles,unsigned capacity){
    ph_editor *e=(ph_editor *)context;unsigned local;ps_settings_editor *editor=local_editor(e,id,&local);
    if(!editor)return 0;
    if(e->values[id].numeric){
        unsigned n=ps_text_length(e->values[id].value);if(n>=capacity)return 0;
        memcpy(text,e->values[id].value,n+1);for(unsigned i=0;i<n;++i)roles[i]=PS_INK_NUMBER;return n;
    }
    return ps_settings_editor_text(editor,local,text,roles,capacity);
}
#endif
static int panel_key(ps_session *session,unsigned key,void *context){
    ph_editor *e=(ph_editor *)context;
    if(key==PS_KEY_F4){unsigned id;if(e->open&&ps_panel_selection(session,&id)&&id<e->count)e->open(e->context,e->values[id].name);return 1;}
    if(key!=PS_KEY_F6)return 0;
    if(ps_panel_set_key_names(session,!e->key_names))e->key_names=!e->key_names;return 1;
}
int ph_editor_open(ph_editor *e,ps_session *s){
    static const ps_column columns[]={{"Control",28,48,1},{"Value",16,0,2}};
    for(unsigned p=0;p<e->page_count;++p){
        ph_page *page=e->sections+p;
        ps_settings_init(&page->registry,e->entries+page->start,page->count);
        page->registry.count=page->count;
        if(!ps_settings_editor_init(&page->editor,&page->registry,e->controls+page->start,
            e->rows+page->start,page->count,e->pages[p].title))return 0;
        if(!ps_settings_editor_set_presentation(&page->editor,e->metadata+page->start,page->count))return 0;
        e->pages[p]=page->editor.page;
        for(unsigned i=0;i<page->count;++i)e->controls[page->start+i].id=page->start+i;
    }
    e->panel.title="Settings";e->panel.status="";e->panel.pages=e->pages;e->panel.count=e->page_count;
    e->panel.read=panel_read;e->panel.write=panel_write;e->panel.context=e;
    e->chrome.columns=columns;e->chrome.count=2;e->chrome.navigation="Tab Pages  F4 Native editor  F6 Names/keys  Esc Close";
#if PS_PANEL_TEXT
    e->chrome.text=panel_text;
#endif
    e->presentation.label=panel_label;e->presentation.key=panel_key_name;e->presentation.choice=panel_choice;e->presentation.context=e;
    if(!ps_panel_attach(s,&e->panel))return 0;
    ps_panel_set_chrome(s,&e->chrome);ps_panel_set_presentation(s,&e->presentation);
    ps_panel_set_key_handler(s,panel_key,e);ps_panel_set_key_names(s,e->key_names);
    return ps_panel_open(s,0);
}
void ph_shell_style(ps_session *s){
    ps_set_semantic_accent(s,1,(1u<<PS_INK_HEADING)|(1u<<PS_INK_ACTION)|(1u<<PS_INK_COMMAND));
    ps_set_dialog_style(s,PS_DIALOG_TITLE_BAR|PS_DIALOG_FRAMED_BUTTONS);
}
void ph_help_setup(ps_session *s){
    static const ps_help_item args[]={{"KEY","Stable setting key; Tab completes names"},{"VALUE","Validated value; quote JSON or text containing spaces"}};
    static const ps_help_item examples[]={{"configure","Edit Preferences in-place; Tab changes section, F6 names/keys"},{"get ui.terminal.font_size","Read the actual host value"},{"set ui.terminal.font_size 14","Apply and save through the same owner as Preferences"}};
    static const ps_help_document docs[]={
        {"configure","Real Pealayer Preferences, not a second configuration store. Changes elsewhere refresh the values.",0,0,0,0,examples,1},
        {"get","Inspect settings. Private values are available only in native Preferences.",args,1,0,0,examples+1,1},
        {"set","Validate, apply and save. Stale edits are rejected. Shells cannot grant execution rights. Complex values use JSON or their native editor.",args,2,0,0,examples+2,1},
        {"help","Tab completes commands, F7 opens history, Ctrl-L redraws, Ctrl-C cancels input.",0,0,0,0,0,0}
    };
    ps_set_help_documents(s,docs,sizeof docs/sizeof docs[0]);
}
