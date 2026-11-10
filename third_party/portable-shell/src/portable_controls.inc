/* Private implementation: shared renderer/editor primitives, no OS/runtime imports. */
static const char *control_label(ps_session *s,const ps_control *c){
#if PS_PANEL_PRESENTATION
    const ps_panel_presentation *p=s->panel_presentation;const char *text=0;
    if(p){ps_panel_name_fn fn=s->panel_key_names?p->key:p->label;if(fn)text=fn(p->context,c->id);}
    if(text)return text;
#else
    (void)s;
#endif
    return c->label;
}
static const char *control_choice_label(ps_session *s,const ps_control *c,unsigned index){
#if PS_PANEL_PRESENTATION
    const ps_panel_presentation *p=s->panel_presentation;const char *text=0;
    if(p&&p->choice)text=p->choice(p->context,c->id,index);
    if(text)return text;
#else
    (void)s;
#endif
    return c->choices[index];
}
PS_API void ps_panel_set_presentation(ps_session *s,const ps_panel_presentation *p){
#if PS_PANEL_PRESENTATION
    if(s&&!s->editing){s->panel_presentation=p;s->dirty=1;}
#else
    (void)s;(void)p;
#endif
}
PS_API int ps_panel_set_key_names(ps_session *s,int enabled){
#if PS_PANEL_PRESENTATION
    if(s&&!s->editing){s->panel_key_names=!!enabled;s->dirty=1;return 1;}
#else
    (void)s;(void)enabled;
#endif
    return 0;
}
static int control_sections(const ps_session *s) {
#if PS_DASHBOARD
    if(s->dashboard)return 0;
#endif
    return PS_CONTROL_SECTION_ROWS && !(s->table_style & PS_TABLE_CATEGORY_COLUMN);
}
/* Encoded presentation rows keep provider indices stable. Headings never write. */
static unsigned control_items(ps_session *s, unsigned short *items, unsigned *focus, unsigned frame) {
    const ps_control_page *p = &s->panel->pages[s->tab];
    unsigned i, n = 0, closed = 0, start = 0;
    *focus = 0;
    for (i = 0; i < p->count; ++i) {
        const char *group = p->controls[i].group;
        if (control_sections(s) && (!i || !equal(group,p->controls[i-1].group))) {
            start = i;
            closed = !(s->table_style&PS_TABLE_STATIC_SECTIONS) && *group && (s->control_closed[s->tab][i/8] & (1U << (i%8)));
            if (*group) {
                /* Reuse an adjacent rule (column header or closed section). */
                if (frame && !table_border_titles(s) && !(s->table_style & PS_TABLE_COMPACT_SECTIONS) && n && !(items[n-1]&0x2000)) items[n++] = (unsigned short)(i | 0x4000);
                if (s->selected == i && s->control_header) *focus = n;
                items[n++] = (unsigned short)(i | 0x8000);
                if (frame && !(s->table_style & PS_TABLE_COMPACT_SECTIONS) && (s->table_style & PS_TABLE_SECTION_BOTTOM)) items[n++] = (unsigned short)(i | 0x2000);
            }
        }
        if (closed) {
            if (i == s->selected) { s->selected = start; s->control_header = 1; *focus = n-1-(frame && !(s->table_style & PS_TABLE_COMPACT_SECTIONS) && (s->table_style & PS_TABLE_SECTION_BOTTOM)); }
        } else {
            if (i == s->selected && !s->control_header) *focus = n;
            items[n++] = (unsigned short)i;
        }
    }
    return n;
}
static int control_navigate(ps_session *s, unsigned key, unsigned step) {
    unsigned short items[256]; unsigned focus, n;
    if (!control_sections(s)) return 0;
    n = control_items(s,items,&focus,0);
    if (!n) return 0;
    if (!(s->table_style&PS_TABLE_STATIC_SECTIONS) && s->control_header && (key == 13 || key == ' ' || key == PS_KEY_LEFT || key == PS_KEY_RIGHT)) {
        ps_byte *bits = &s->control_closed[s->tab][s->selected/8];
        unsigned mask = 1U << (s->selected%8);
        if (key == PS_KEY_LEFT) *bits |= (ps_byte)mask;
        else if (key == PS_KEY_RIGHT) *bits &= (ps_byte)~mask;
        else *bits ^= (ps_byte)mask;
        return 1;
    }
    if (key == PS_KEY_DOWN || key == 'j') focus = min(focus+1,n-1);
    else if (key == PS_KEY_UP || key == 'k') focus = focus ? focus-1 : 0;
    else if (key == PS_KEY_PAGEDOWN) focus = min(focus+step,n-1);
    else if (key == PS_KEY_PAGEUP) focus = focus > step ? focus-step : 0;
    else return 0;
    s->selected = items[focus] & 0x7fff;
    s->control_header = (items[focus] & 0x8000) != 0;
    return 1;
}
static void control_section_row(ps_session *s, unsigned index, unsigned width) {
    char label[128]; unsigned i;
    const char *name = s->panel->pages[s->tab].controls[index].group;
    unsigned closed = s->control_closed[s->tab][index/8] & (1U << (index%8));
    for (i=0; name[i] && i+1<sizeof label; ++i)
        label[i] = name[i]>='a' && name[i]<='z' ? (char)(name[i]-'a'+'A') : name[i];
    label[i]=0;
    {
        const char *marker=ps_table_section_marker(!!(s->io.flags&PS_UNICODE),
            !(s->table_style&PS_TABLE_STATIC_SECTIONS),
            s->table_style&PS_TABLE_SECTION_NO_ICONS?2:s->table_style&PS_TABLE_SECTION_PLUS_MINUS?1:0,closed,
            s->control_header&&s->selected==index,!!(s->table_style&PS_TABLE_SECTION_INDICATORS_ALWAYS));
        unsigned marker_width=cells(marker,length(marker),s->io.flags);
        if(table_border_titles(s)) {
            if(s->control_header && s->selected==index)surface(s,1);else ink(s,PS_INK_HEADING);
            if(s->control_header && s->selected==index && !(s->io.flags&PS_COLOR))ansi(s,"\033[7m");
            out(s,marker);grid_text(s,label,min(width-marker_width-1,cells(label,length(label),s->io.flags)));out(s," ");
        } else {
            surface(s,s->control_header && s->selected==index?1:0);
            out(s,marker);grid_text(s,label,width-marker_width);
        }
    }
}
static const ps_control *control_selected(ps_session *s) {
    const ps_control_page *p = &s->panel->pages[s->tab];
    return !s->control_header && s->selected < p->count ? &p->controls[s->selected] : 0;
}
static int control_read(ps_session *s, const ps_control *c, ps_control_value *v) {
    zero(v, sizeof *v);
    if (!s->panel->read(s->panel->context, c->id, v)) {
        zero(v, sizeof *v);
        return 0;
    }
    v->text[63] = 0;
    if (c->kind == PS_CONTROL_LEVEL && (v->number < c->minimum || v->number > c->maximum))
        v->flags &= ~PS_CONTROL_VALID;
    if ((c->kind == PS_CONTROL_TOGGLE && v->number > 1) ||
        (c->kind == PS_CONTROL_CHOICE && (!c->choices||!c->choice_count||c->choice_count>128||v->number >= c->choice_count||!c->choices[v->number])))
        v->flags &= ~PS_CONTROL_VALID;
    return (v->flags & PS_CONTROL_VALID) != 0;
}
static int control_writable(ps_session *s, const ps_control *c, ps_control_value *v) {
    control_read(s, c, v);
    if (!(v->flags & PS_CONTROL_VALID)) {
        notice(s, "State unknown");
        return 0;
    }
    if (v->flags & PS_CONTROL_PENDING) {
        notice(s, "Waiting for readback");
        return 0;
    }
    if (!s->panel->write || (v->flags & PS_CONTROL_LOCKED) || c->kind == PS_CONTROL_LABEL) {
        notice(s, "Read only");
        return 0;
    }
    return 1;
}
static int control_decimal(const char *p, unsigned *n) {
    unsigned value = 0;
    if (!*p)
        return 0;
    while (*p) {
        if (*p < '0' || *p > '9' || value > 6553 || (value == 6553 && *p > '5'))
            return 0;
        value = value * 10 + (unsigned)(*p++ - '0');
    }
    *n = value;
    return 1;
}
static int control_validate(const ps_control *c, const ps_control_value *v) {
    unsigned i;
    if (c->kind == PS_CONTROL_TOGGLE)
        return v->number <= 1;
    if (c->kind == PS_CONTROL_LEVEL)
        return v->number >= c->minimum && v->number <= c->maximum;
    if (c->kind == PS_CONTROL_CHOICE)
        return v->number < c->choice_count;
    for (i = 0; i < sizeof v->text; ++i) {
        if (!v->text[i])
            return 1;
        if ((ps_byte)v->text[i] < 32 || (ps_byte)v->text[i] == 127)
            return 0;
    }
    return 0;
}
static int control_commit(ps_session *s, const ps_control *c, ps_control_value *request,
                          ps_u32 expected) {
    ps_control_value current;
    int result;
    if (!control_validate(c, request)) {
        notice(s, "Invalid value or outside range");
        return 0;
    }
    if (!control_writable(s, c, &current))
        return 0;
    if (current.revision != expected) {
        notice(s, "State changed; cancel and reopen editor");
        return 0;
    }
    result = s->panel->write(s->panel->context, c->id, request, expected);
    notice(s, result == 1 ? "Applied" : result == 2 ? "Pending readback" : "Rejected");
    s->dirty = 1;
    return result == 1 || result == 2;
}
static void control_value_text(ps_session *s, const ps_control *c, const ps_control_value *v,
                               char *text, unsigned cap) {
    if (!(v->flags & PS_CONTROL_VALID))
        str(text, "Unknown", cap);
    else if (c->kind == PS_CONTROL_TOGGLE)
        str(text,
            (s->io.flags & PS_UNICODE) ? (v->number ? "● On" : "○ Off")
                                       : (v->number ? "[x] On" : "[ ] Off"),
            cap);
    else if (c->kind == PS_CONTROL_CHOICE)
        str(text, control_choice_label(s,c,v->number), cap);
    else if (c->kind == PS_CONTROL_LEVEL)
        dec(text, v->number);
    else if (c->kind == PS_CONTROL_ACTION)
        str(text, v->text[0] ? v->text : "Run", cap);
    else
        str(text, v->text, cap);
}
#if PS_ACTION_BUTTONS
static void control_button(ps_session *s, const ps_control_value *v, const char *label,
                           unsigned width, unsigned focused) {
    enum { BUTTON_PADDING = 2, BUTTON_FRAME = 2 * BUTTON_PADDING };
    unsigned state=focused?PS_BUTTON_FOCUSED:PS_BUTTON_IDLE;
    unsigned span=min(width,cells(label,length(label),s->io.flags)+BUTTON_FRAME);
    const char *status="";
    if (!(v->flags&PS_CONTROL_VALID)) {state=PS_BUTTON_DISABLED;status="Unknown";}
    else if (v->flags&PS_CONTROL_PENDING) {state=PS_BUTTON_PENDING;status="Pending";}
    else if ((v->flags&PS_CONTROL_LOCKED)||!s->panel->write) {state=PS_BUTTON_DISABLED;status="Read only";}
    if(span<BUTTON_FRAME){grid_text(s,label,width);return;}
    ansi(s,"\033[0m");
    if(s->io.flags&PS_COLOR)color(s,ps_style_button(s->theme,!!(s->io.flags&PS_TRUECOLOR),state));
    else ansi(s,state==PS_BUTTON_FOCUSED?"\033[1;7m":state==PS_BUTTON_DISABLED?"\033[2m":"\033[1m");
    out(s,s->io.flags&PS_COLOR?"  ":"[ ");
    grid_text(s,label,span-BUTTON_FRAME);
    out(s,s->io.flags&PS_COLOR?"  ":" ]");
    /* Restore the row, so padding and the table border never inherit the button. */
    reset(s);
    if(focused){if(s->io.flags&PS_COLOR)surface(s,2);else ansi(s,"\033[7m");}
    ink(s,state==PS_BUTTON_PENDING?PS_INK_WARNING:PS_INK_MUTED);
    if(*status&&width-span>2){out(s,"  ");grid_text(s,status,width-span-2);}
    else spaces(s,width-span);
}
#endif
#if PS_PANEL_KEYS && PS_PANEL_TEXT
static int control_styled_text(ps_session *s,unsigned id,unsigned columns) {
    char text[PS_PANEL_TEXT_BYTES];ps_byte roles[PS_PANEL_TEXT_BYTES];
    unsigned n,at=0,end,used=0,limit=columns,suffix=(s->io.flags&PS_UNICODE)?1:3,clipped;
    if(!s->panel_chrome||!s->panel_chrome->text)return 0;
    zero(roles,sizeof roles);
    n=s->panel_chrome->text(s->panel->context,id,text,roles,sizeof text);
    if(!n||n>=sizeof text)return 0;
    text[n]=0;clipped=cells(text,n,s->io.flags)>columns&&columns>=suffix;
    if(clipped)limit-=suffix;
    while(at<n&&used<limit){unsigned span;char saved;
        end=next(text,at);
        while(end<n&&roles[end]==roles[at])end=next(text,end);
        saved=text[end];text[end]=0;
        span=min(cells(text+at,end-at,s->io.flags),limit-used);
        ink(s,roles[at]<=PS_INK_ACCENT?roles[at]:PS_INK_TEXT);
        field(s,text+at,span);used+=span;text[end]=saved;at=end;
    }
    ink(s,PS_INK_MUTED);spaces(s,limit-used);
    if(clipped)out(s,(s->io.flags&PS_UNICODE)?"…":"...");
    return 1;
}
#endif
static void control_row(ps_session *s, unsigned index, const Grid *g) {
    const ps_control *c = &s->panel->pages[s->tab].controls[index];
    ps_control_value v;
    unsigned col = 0, w, i, pct;
    char b[80];
    control_read(s, c, &v);
    if (g->count == 3) {
        ink(s, 0);
        const ps_control_page *page = &s->panel->pages[s->tab];
        grid_text(s, index == s->scroll || !index || !equal(c->group, page->controls[index-1].group)
                         ? c->group : "", g->widths[col++]);
        grid_divide(s);
    }
    ink(s, v.flags&PS_CONTROL_ATTENTION?PS_INK_WARNING:v.flags&PS_CONTROL_INACTIVE?PS_INK_INACTIVE:PS_INK_TEXT);
#if PS_PANEL_PRESENTATION
    if(s->panel_key_names)ink(s,PS_INK_COMMAND);
#endif
    {
        const ps_control_page *page=&s->panel->pages[s->tab];unsigned j,categories=0;
        if(control_sections(s))for(j=0;j<page->count;++j)if(*page->controls[j].group){categories=1;break;}
        const char *text=ps_table_marker(s->selection_marker_override?s->selection_marker:0,
            !!(s->io.flags&PS_UNICODE),categories,!(s->table_style&PS_TABLE_STATIC_SECTIONS),
            (s->io.flags&PS_UNICODE)&&!(s->table_style&(PS_TABLE_SECTION_PLUS_MINUS|PS_TABLE_SECTION_NO_ICONS)));
        unsigned marker=min(cells(text,length(text),s->io.flags),g->widths[col]);
        field(s,!s->control_header && s->selected==index?text:"",marker);
        grid_text(s,control_label(s,c),g->widths[col++]-marker);
    }
    grid_divide(s);
    w = g->widths[col];
    ink(s, !(v.flags & PS_CONTROL_VALID)  ? 0
           : v.flags & PS_CONTROL_INACTIVE ? PS_INK_INACTIVE
           : c->kind == PS_CONTROL_TOGGLE ? (v.number ? PS_INK_SUCCESS : s->theme==PS_THEME_DEFAULT?PS_INK_INACTIVE:3)
           : c->kind == PS_CONTROL_ACTION ? PS_INK_ACTION
           : c->kind == PS_CONTROL_LEVEL ? PS_INK_NUMBER
           : c->kind == PS_CONTROL_CHOICE ? PS_INK_COMMAND
           : c->kind == PS_CONTROL_LABEL ? PS_INK_TEXT : PS_INK_STRING);
    if (s->editing == 5 && !s->dialog && index == s->selected) {
        unsigned start = 0;
        while (start < s->editpos && cells(s->edit + start, s->editpos - start, s->io.flags) >= w)
            start = next(s->edit, start);
        surface(s, 1);
        /* The table divider already supplies the cell's padding. */
        field(s, s->edit + start, w);
        return;
    }
#if PS_ACTION_BUTTONS
    if(c->kind==PS_CONTROL_ACTION){
        /* Keep the action name visible even when its state is unknown. */
        str(b,v.text[0]?v.text:"Run",sizeof b);
        control_button(s,&v,b,w,!s->control_header&&s->selected==index);
        return;
    }
#endif
#if PS_PANEL_KEYS && PS_PANEL_TEXT
    if((v.flags&PS_CONTROL_VALID)&&!(v.flags&(PS_CONTROL_PENDING|PS_CONTROL_LOCKED))&&
       (c->kind==PS_CONTROL_TEXT||c->kind==PS_CONTROL_LABEL)&&control_styled_text(s,c->id,w))return;
#endif
    if ((v.flags & PS_CONTROL_VALID) && !(v.flags & (PS_CONTROL_PENDING | PS_CONTROL_LOCKED | PS_CONTROL_NUMBER_ONLY)) &&
        c->kind == PS_CONTROL_LEVEL && w >= 18) {
        unsigned bar = w - 15;
        pct = (v.number - c->minimum) * 100 / (c->maximum - c->minimum);
        for (i = 0; i < bar; ++i) {
            ink(s, i * 100 < pct * bar ? PS_INK_NUMBER : PS_INK_MUTED);
            out(s, (s->io.flags & PS_UNICODE) ? (i * 100 < pct * bar ? "━" : "─")
                                              : (i * 100 < pct * bar ? "#" : "."));
        }
        ink(s, PS_INK_NUMBER);
        out(s, " ");
        dec(b, pct);
        str(b + length(b), "%", sizeof b - length(b));
        field(s, b, 5);
        dec(b, v.number);
        str(b + length(b), "/", sizeof b - length(b));
        dec(b + length(b), c->maximum);
        field(s, b, 9);
    } else {
        control_value_text(s, c, &v, b, sizeof b);
        if (v.flags & PS_CONTROL_PENDING) {
            ink(s, s->theme==PS_THEME_DEFAULT?PS_INK_WARNING:6);
            str(b + length(b), " [pending]", sizeof b - length(b));
        } else if (v.flags & PS_CONTROL_LOCKED)
            str(b + length(b), " [locked]", sizeof b - length(b));
        grid_text(s, b, w);
    }
}
static const char *control_hint(ps_session *s) {
    const ps_control *c = control_selected(s);
    ps_control_value v;
    if (s->control_header && (s->table_style&PS_TABLE_STATIC_SECTIONS)) return "";
    if (s->control_header)
        return (s->io.flags & PS_UNICODE) ? "Enter/Space toggle  ← collapse  → expand"
                                         : "Enter/Space toggle  Left close Right open";
#if PS_CONTROL_CHOICE_POPUP
    if(s->editing&&s->choice_popup)return (s->io.flags&PS_UNICODE)?"↑/↓ select  Enter apply  Esc cancel":"Up/Down select Enter apply Esc cancel";
#endif
    if (s->editing)
        return s->dialog ? "Tab focus   Enter apply   Esc cancel"
                         : "Enter apply  Esc cancel  ^U clear";
#if PS_PANEL_KEYS
    if(s->panel_chrome&&s->panel_chrome->actions)return s->panel_chrome->actions;
#endif
    if (!c)
        return "";
    control_read(s, c, &v);
    if (!(v.flags & PS_CONTROL_VALID))
        return "Waiting for state";
    if (v.flags & PS_CONTROL_PENDING)
        return "Waiting for readback";
    if ((v.flags & PS_CONTROL_LOCKED) || !s->panel->write)
        return "Read only";
    if (c->kind == PS_CONTROL_LEVEL)
        return (s->io.flags & PS_UNICODE) ? "←/→ adjust  Enter value  F2 dialog"
                                          : "Left/Right adjust  Enter value  F2 dialog";
    if (c->kind == PS_CONTROL_TOGGLE)
        return s->io.columns < 60 ? "Space toggle  Enter toggle  F2 edit"
                                  : "Space toggle   Enter toggle   F2 dialog";
    if (c->kind == PS_CONTROL_CHOICE)
        return (s->io.flags & PS_UNICODE) ? "←/→ choose   Enter/F2 options"
                                          : "Left/Right choose   F2 dialog";
    if (c->kind == PS_CONTROL_ACTION)
        return "Enter open confirmation";
    if (c->kind == PS_CONTROL_TEXT)
        return "Enter inline edit   F2 dialog";
    return "Read only";
}
#if PS_CONTROL_CHOICE_POPUP
enum { CHOICE_VISIBLE_MAX=12, CHOICE_WIDTH_MIN=44, CHOICE_INLINE_WIDTH_MIN=28, CHOICE_WIDTH_MAX=72,
       CHOICE_RESERVED_ROWS=10, CHOICE_PADDING=8 };
typedef struct {unsigned x,y,width,visible,start,extra;} ChoiceBox;
static ps_u32 control_choice_schema(ps_session *s,const ps_control *c){
    ps_u32 h=2166136261u;unsigned i,j;
    for(i=0;i<c->choice_count;++i){
        const char *texts[2]={c->choices[i],control_choice_label(s,c,i)};
        for(j=0;j<PS_COUNT_OF(texts);++j){const char *p=texts[j];
            while(*p){h^=(ps_byte)*p++;h*=16777619u;}h*=16777619u;}
    }
    return h^c->choice_count;
}
static int control_choice_valid(ps_session *s){
    const ps_control *c=control_selected(s);ps_control_value v;unsigned i;
    if(c&&c->id==s->choice_id&&c->kind==PS_CONTROL_CHOICE&&c->choices&&
       c->choice_count&&c->choice_count<=128&&s->choice_index<c->choice_count){
        for(i=0;i<c->choice_count&&c->choices[i];++i){}
        if(i==c->choice_count&&control_read(s,c,&v)&&s->panel->write&&
           !(v.flags&(PS_CONTROL_LOCKED|PS_CONTROL_PENDING))&&v.revision==s->draft_revision&&
           v.number==s->choice_initial&&control_choice_schema(s,c)==s->choice_schema)return 1;
    }
    s->editing=s->dialog=s->choice_popup=0;
    notice(s,"State changed; reopen options");return 0;
}
static ChoiceBox control_choice_box(ps_session *s,const ps_control *c){
    ChoiceBox b;unsigned i,w=s->choice_popup==1?CHOICE_INLINE_WIDTH_MIN:CHOICE_WIDTH_MIN;
    for(i=0;i<c->choice_count;++i){const char *p=control_choice_label(s,c,i);
        unsigned n=cells(p,length(p),s->io.flags)+CHOICE_PADDING;if(n>w)w=n;}
    b.width=min(min(w,CHOICE_WIDTH_MAX),s->io.columns>4?s->io.columns-4:0);
    b.visible=min(c->choice_count,min(CHOICE_VISIBLE_MAX,s->io.rows>CHOICE_RESERVED_ROWS?s->io.rows-CHOICE_RESERVED_ROWS:1));
    b.start=s->choice_index>=b.visible?s->choice_index-b.visible+1:0;
    b.start=min(b.start,c->choice_count-b.visible);
    b.extra=(c->choice_count>b.visible?1u:0u)+(s->notice[0]?1u:0u);
    b.x=(s->io.columns-b.width)/2+1;b.y=s->io.rows>b.visible+4?(s->io.rows-b.visible-4)/2+1:1;
    /* Enter/Space attaches the list to its value cell; F2 keeps the modal view. */
    if(s->choice_popup==1&&s->mouse_edit_x&&s->mouse_edit_y){
        unsigned height=b.visible+2+b.extra,bottom=s->io.rows>4?s->io.rows-4:1;
        b.x=min(s->mouse_edit_x,s->io.columns-b.width+1);
        if(s->mouse_edit_y+height<bottom)b.y=s->mouse_edit_y+1;
        else b.y=s->mouse_edit_y>height?s->mouse_edit_y-height:1;
    }
    return b;
}
static void control_choice_key(ps_session *s,unsigned key){
    const ps_control *c;ps_control_value v;ChoiceBox b;
    if(!control_choice_valid(s))return;
    c=control_selected(s);b=control_choice_box(s,c);s->dirty=1;
    if(key==27||key==3){s->editing=s->dialog=s->choice_popup=0;notice(s,"Cancelled");}
    else if(key==13){zero(&v,sizeof v);v.number=s->choice_index;
        if(v.number==s->choice_initial||control_commit(s,c,&v,s->draft_revision))s->editing=s->dialog=s->choice_popup=0;
    }else if(key==PS_KEY_UP||key==PS_KEY_BACKTAB)s->choice_index=(s->choice_index+c->choice_count-1)%c->choice_count;
    else if(key==PS_KEY_DOWN||key==9||key==' ')s->choice_index=(s->choice_index+1)%c->choice_count;
    else if(key==PS_KEY_HOME)s->choice_index=0;
    else if(key==PS_KEY_END)s->choice_index=c->choice_count-1;
    else if(key==PS_KEY_PAGEUP)s->choice_index=s->choice_index>b.visible?s->choice_index-b.visible:0;
    else if(key==PS_KEY_PAGEDOWN)s->choice_index=min(s->choice_index+b.visible,c->choice_count-1);
}
static void control_choice_draw(ps_session *s){
    const ps_control *c;ChoiceBox b;unsigned i,j,unicode=!!(s->io.flags&PS_UNICODE);
    if(!control_choice_valid(s))return;
    c=control_selected(s);b=control_choice_box(s,c);
    for(i=0;i<b.visible+2+b.extra;++i){
        at(s,b.y+i,b.x);surface(s,2);ink(s,PS_INK_BORDER);
        if(i==0||i==b.visible+1){
            out(s,unicode?(i?"╰":"╭") : "+");
            for(j=0;j<b.width-2;++j)out(s,unicode?"─":"-");
            out(s,unicode?(i?"╯":"╮") : "+");
        }else if(i<=b.visible){unsigned index=b.start+i-1,selected=index==s->choice_index;
            out(s,unicode?"│":"|");
            if(selected){surface(s,1);if(!(s->io.flags&PS_COLOR))ansi(s,"\033[7m");}
            ink(s,selected?PS_INK_ACCENT:PS_INK_MUTED);out(s,selected?(unicode?" ❯ ":" > "):"   ");
            /* The marker identifies focus; the circle identifies the saved choice. */
            ink(s,index==s->choice_initial?PS_INK_SUCCESS:PS_INK_TEXT);
            out(s,index==s->choice_initial?(unicode?"● ":"* "):"  ");
            if(selected)color(s,"\033[30m");else ink(s,PS_INK_TEXT);
            grid_text(s,control_choice_label(s,c,index),b.width-8);
            surface(s,2);ink(s,PS_INK_BORDER);out(s," ");
            if(c->choice_count>b.visible){unsigned thumb=b.visible*b.visible/c->choice_count;
                if(!thumb)thumb=1;
                unsigned top=b.start*(b.visible-thumb)/(c->choice_count-b.visible);
                ink(s,PS_INK_ACCENT);out(s,unicode?(i-1>=top&&i-1<top+thumb?"┃":"│"):(i-1>=top&&i-1<top+thumb?"#":"|"));
            }else out(s,unicode?"│":"|");
        }else{char range[32];ps_terminal_format(range,sizeof range,"%u-%u / %u",b.start+1,b.start+b.visible,c->choice_count);
            ink(s,PS_INK_MUTED);grid_text(s,i==b.visible+2&&c->choice_count>b.visible?range:s->notice,b.width);}
        reset(s);
    }
    at(s,b.y,b.x+2);surface(s,2);ink(s,PS_INK_ACCENT);out(s," ");grid_text(s,control_label(s,c),min(b.width-6,cells(control_label(s,c),length(control_label(s,c)),s->io.flags)));out(s," ");reset(s);
    ansi(s,"\033[?25l");
}
#if PS_MOUSE
static void control_choice_mouse(ps_session *s,const ps_mouse_event *e){
    ChoiceBox b;const ps_control *c;unsigned button=e->button&3;
    if(!control_choice_valid(s))return;
    if(e->button&64u){if(button<2)control_choice_key(s,button?PS_KEY_DOWN:PS_KEY_UP);return;}
    if(e->button&32u)return;
    if(button==2){control_choice_key(s,27);return;}
    if(button)return;
    c=control_selected(s);b=control_choice_box(s,c);
    if(e->column>b.x&&e->column<b.x+b.width-1&&e->row>b.y&&e->row<=b.y+b.visible){
        s->choice_index=b.start+e->row-b.y-1;control_choice_key(s,13);
    }
}
#endif
#endif
static void control_overlay(ps_session *s, const Grid *table) {
    const ps_control *c = control_selected(s);
    unsigned w, x, y, i, start = 0, cursor_x, cursor_y;
    ps_control_value v;
    char b[80];
    if (!c || !s->editing)
        return;
#if PS_CONTROL_CHOICE_POPUP
    if(s->choice_popup){
#if PS_DASHBOARD
        if(!s->dashboard)
#endif
        {s->mouse_edit_x=4;s->mouse_edit_y=s->control_edit_row;
            for(i=0;i+1<table->count;++i)s->mouse_edit_x+=table->widths[i]+3;
            s->mouse_edit_width=table->widths[table->count-1];}
        control_choice_draw(s);return;
    }
#endif
    if (!s->dialog) {
        w = table->widths[table->count - 1];
        x = 4;
        for (i = 0; i + 1 < table->count; ++i)
            x += table->widths[i] + 3;
        while (start < s->editpos && cells(s->edit + start, s->editpos - start, s->io.flags) >= w)
            start = next(s->edit, start);
        s->mouse_edit_x=x;s->mouse_edit_y=s->control_edit_row;
        s->mouse_edit_start=start;s->mouse_edit_width=w;
        at(s, s->control_edit_row,
           x + cells(s->edit + start, s->editpos - start, s->io.flags));
        ansi(s, "\033[?25h");
        return;
    }
    w = min(60, s->io.columns - 6);
    x = (s->io.columns - w) / 2 + 1;
    y = (s->io.rows - 9) / 2 + 1;
    for (i = 0; i < 9; ++i) {
        at(s, y + i, x);
        surface(s, 2);
        ink(s, 7);
        out(s, (s->io.flags & PS_UNICODE) ? (i == 0 ? "╭" : i == 8 ? "╰" : "│") : "+");
        if (i == 0 || i == 8) {
            unsigned j;
            for (j = 0; j < w - 2; ++j)
                out(s, (s->io.flags & PS_UNICODE) ? "─" : "-");
        } else
            spaces(s, w - 2);
        out(s, (s->io.flags & PS_UNICODE) ? (i == 0 ? "╮" : i == 8 ? "╯" : "│") : "+");
    }
    at(s, y + 1, x + 3);
    ink(s, 1);
    grid_text(s, control_label(s,c), w - 6);
    at(s, y + 2, x + 3);
    ink(s, 0);
    grid_text(s, c->group, w - 6);
    control_read(s, c, &v);
    control_value_text(s, c, &v, b, sizeof b);
    at(s, y + 3, x + 3);
    ink(s, 4);
    out(s, "Observed  ");
    grid_text(s, b, w - 16);
    at(s, y + 4, x + 3);
    if (c->kind == PS_CONTROL_ACTION) {
        ink(s, 6);
        grid_text(s, "Confirm action?", w - 6);
    } else {
        ink(s, 4);
        out(s, "Draft     ");
        while (cells(s->edit + start, s->editpos - start, s->io.flags) >= w - 17)
            start = next(s->edit, start);
        if (!s->dialog_focus)
            surface(s, 1);
        field(s, s->edit + start, w - 17);
        surface(s, 2);
    }
    cursor_x = x + 13 + cells(s->edit + start, s->editpos - start, s->io.flags);
    cursor_y = y + 4;
    s->mouse_edit_x=x+13;s->mouse_edit_y=cursor_y;
    s->mouse_edit_start=start;s->mouse_edit_width=w-17;
    at(s, y + 5, x + 3);
    ink(s, 0);
    if (c->kind == PS_CONTROL_LEVEL) {
        ps_number(s, c->minimum);
        out(s, "..");
        ps_number(s, c->maximum);
        out(s, "  step ");
        ps_number(s, c->step);
    } else if (c->kind == PS_CONTROL_CHOICE) {
        unsigned choice;
        if (control_decimal(s->edit, &choice) && choice < c->choice_count)
            grid_text(s, c->choices[choice], w - 6);
    } else if (c->kind == PS_CONTROL_TOGGLE)
        out(s, "0 Off   1 On");
    at(s, y + 7, x + 3);
    if (s->dialog_focus == 1)
        surface(s, 1);
    ink(s, 5);
    out(s, s->dialog_focus == 1 ? "[ Apply ]" : "  Apply  ");
    surface(s, 2);
    out(s, "   ");
    if (s->dialog_focus == 2)
        surface(s, 1);
    ink(s, 4);
    out(s, s->dialog_focus == 2 ? "[ Cancel ]" : "  Cancel  ");
    reset(s);
    if (!s->dialog_focus && c->kind != PS_CONTROL_ACTION) {
        at(s, cursor_y, cursor_x);
        ansi(s, "\033[?25h");
    }
}
static void control_begin(ps_session *s, const ps_control *c, unsigned dialog) {
    ps_control_value v;
    char b[64];
    if (!control_writable(s, c, &v))
        return;
    if (c->kind == PS_CONTROL_TEXT)
        str(b, v.text, sizeof b);
    else
        dec(b, v.number);
    enter_edit(s, 5, b);
    s->draft_revision = v.revision;
    s->notice[0] = 0;
    s->dialog = dialog || c->kind == PS_CONTROL_ACTION;
    s->dialog_focus = c->kind == PS_CONTROL_ACTION ? 2 : 0;
#if PS_CONTROL_CHOICE_POPUP
    s->choice_popup=c->kind==PS_CONTROL_CHOICE?(dialog?2u:1u):0u;
    if(s->choice_popup){s->dialog=1;s->choice_id=c->id;s->choice_index=s->choice_initial=v.number;
        s->choice_schema=control_choice_schema(s,c);}
#endif
}
static void control_key(ps_session *s, unsigned key) {
    const ps_control *c = control_selected(s);
    unsigned n = tab_count(s), step = table_slots(s);
    ps_control_value v;
    s->dirty = 1;
#if PS_CONTROL_CHOICE_POPUP
    if(s->editing&&s->choice_popup){control_choice_key(s,key);return;}
#endif
    if (s->editing && c) {
        if (key == 27 || key == 3) {
            s->editing = s->dialog = 0;
            notice(s, "Cancelled");
        } else if (s->dialog && (key == 9 || key == PS_KEY_BACKTAB)) {
            unsigned first = c->kind == PS_CONTROL_ACTION ? 1 : 0;
            s->dialog_focus = key == 9 ? (s->dialog_focus == 2 ? first : s->dialog_focus + 1)
                                       : (s->dialog_focus == first ? 2 : s->dialog_focus - 1);
        } else if (key == 13) {
            if (s->dialog && s->dialog_focus == 2) {
                s->editing = s->dialog = 0;
                notice(s, "Cancelled");
                return;
            }
            zero(&v, sizeof v);
            if (c->kind == PS_CONTROL_TEXT)
                str(v.text, s->edit, sizeof v.text);
            else if (c->kind != PS_CONTROL_ACTION && !control_decimal(s->edit, &v.number)) {
                notice(s, "Enter a whole number in range");
                return;
            }
            if (control_commit(s, c, &v, s->draft_revision))
                s->editing = s->dialog = 0;
        } else if (s->dialog_focus && (key == PS_KEY_LEFT || key == PS_KEY_RIGHT)) {
            s->dialog_focus = s->dialog_focus == 1 ? 2 : 1;
        } else if (!s->dialog_focus) {
            if (key == PS_KEY_LEFT)
                s->editpos = prev(s->edit, s->editpos);
            else if (key == PS_KEY_RIGHT)
                s->editpos = next(s->edit, s->editpos);
            else if (key == PS_KEY_HOME || key == 1)
                s->editpos = 0;
            else if (key == PS_KEY_END || key == 5)
                s->editpos = s->editlen;
            else if (key == 21)
                s->edit[0] = 0, s->editpos = s->editlen = 0;
            else if (key == 8 || key == 127)
                edit_delete(s->edit, &s->editlen, &s->editpos, 1);
            else if (key == PS_KEY_DELETE)
                edit_delete(s->edit, &s->editlen, &s->editpos, 0);
            else if (key >= 32 && key < 256)
                edit_insert(s->edit, &s->editlen, &s->editpos, 64, key);
        }
        return;
    }
    if (key == 27 || key == 3 || key == 'q') {
        table_close(s);
        return;
    }
    if (key == 9 || key == PS_KEY_BACKTAB || (key >= '1' && key < '1' + s->panel->count)) {
        s->tab = key == 9                ? (s->tab + 1) % s->panel->count
                 : key == PS_KEY_BACKTAB ? (s->tab + s->panel->count - 1) % s->panel->count
                                         : key - '1';
        s->selected = s->scroll = 0;
        s->control_header = 0;
    } else if (control_navigate(s,key,step)) {
        return;
    } else if (key == PS_KEY_DOWN || key == 'j')
        s->selected = min(s->selected + 1, n ? n - 1 : 0);
    else if (key == PS_KEY_UP || key == 'k')
        s->selected = s->selected ? s->selected - 1 : 0;
    else if (key == PS_KEY_PAGEDOWN)
        s->selected = min(s->selected + step, n ? n - 1 : 0);
    else if (key == PS_KEY_PAGEUP)
        s->selected = s->selected > step ? s->selected - step : 0;
    else if (c) {
        if ((PS_CONTROL_CHOICE_POPUP&&c->kind==PS_CONTROL_CHOICE&&(key==13||key==' ')) || key == PS_KEY_F2 || ((key == 13 || key == ' ') && c->kind != PS_CONTROL_TOGGLE &&
                                 c->kind != PS_CONTROL_CHOICE)) {
            control_begin(s, c, key == PS_KEY_F2);
            return;
        }
        if (c->kind > PS_CONTROL_CHOICE ||
            (key != PS_KEY_LEFT && key != PS_KEY_RIGHT && key != PS_KEY_HOME && key != PS_KEY_END &&
             key != 13 && key != ' '))
            return;
        if (!control_writable(s, c, &v))
            return;
        if (c->kind == PS_CONTROL_LEVEL) {
            if (key == PS_KEY_HOME)
                v.number = c->minimum;
            else if (key == PS_KEY_END)
                v.number = c->maximum;
            else if (key == PS_KEY_LEFT)
                v.number = v.number - c->minimum < c->step ? c->minimum : v.number - c->step;
            else
                v.number = min(v.number + c->step, c->maximum);
        } else if (c->kind == PS_CONTROL_TOGGLE)
            v.number = key == PS_KEY_HOME || key == PS_KEY_LEFT   ? 0
                       : key == PS_KEY_END || key == PS_KEY_RIGHT ? 1
                                                                  : !v.number;
        else
            v.number = key == PS_KEY_HOME   ? 0
                       : key == PS_KEY_END  ? c->choice_count - 1
                       : key == PS_KEY_LEFT ? (v.number + c->choice_count - 1) % c->choice_count
                                            : (v.number + 1) % c->choice_count;
        control_commit(s, c, &v, v.revision);
    }
}
PS_API int ps_panel_attach(ps_session *s, const ps_panel *panel) {
    unsigned p, i, q, j;
    if (!s || s->editing)
        return 0;
    if (panel) {
        if (!panel->title || !panel->status || !panel->pages || !panel->read || !panel->count ||
            panel->count > 8)
            return 0;
        for (p = 0; p < panel->count; ++p) {
            const ps_control_page *page = &panel->pages[p];
            if (!page->title || page->count > 128 || (page->count && !page->controls))
                return 0;
            for (i = 0; i < page->count; ++i) {
                const ps_control *c = &page->controls[i];
                if (!c->label || !c->group || c->kind > PS_CONTROL_LABEL ||
                    (c->kind == PS_CONTROL_LEVEL &&
                     (c->minimum >= c->maximum || c->maximum > 65535 || !c->step ||
                      c->step > 65535)) ||
                    (c->kind == PS_CONTROL_CHOICE &&
                     (!c->choices || !c->choice_count || c->choice_count > 128)))
                    return 0;
                for (j = 0; c->kind == PS_CONTROL_CHOICE && j < c->choice_count; ++j)
                    if (!c->choices[j])
                        return 0;
                for (q = 0; q <= p; ++q)
                    for (j = 0; j < (q == p ? i : panel->pages[q].count); ++j)
                        if (c->id == panel->pages[q].controls[j].id)
                            return 0;
            }
        }
    }
    if (s->control_view)
        table_close(s);
    s->panel = panel;
#if PS_PANEL_PRESENTATION
    s->panel_presentation=0;s->panel_key_names=0;
#endif
#if PS_PANEL_KEYS
    s->panel_chrome=0;s->panel_key=0;s->panel_key_context=0;
#endif
    zero(s->control_closed,sizeof s->control_closed);
    s->control_header = 0;
    return 1;
}
PS_API unsigned ps_panel_active(const ps_session *s){return s&&s->board&&s->control_view?s->tab+1:0;}
PS_API void ps_panel_set_chrome(ps_session *s,const ps_panel_chrome *chrome){
#if PS_PANEL_KEYS
    if(s){s->panel_chrome=chrome;s->dirty=1;}
#else
    (void)s;(void)chrome;
#endif
}
PS_API void ps_panel_set_key_handler(ps_session *s,ps_panel_key_fn fn,void *context){
#if PS_PANEL_KEYS
    if(s){s->panel_key=fn;s->panel_key_context=context;}
#else
    (void)s;(void)fn;(void)context;
#endif
}
PS_API int ps_panel_selection(const ps_session *s,unsigned *id){
    const ps_control_page *p;
    if(!ps_panel_active(s)||s->editing||s->control_header||!id)return 0;
    p=&s->panel->pages[s->tab];
    if(s->selected>=p->count)return 0;
    *id=p->controls[s->selected].id;return 1;
}
PS_API int ps_panel_select(ps_session *s,unsigned id){
    unsigned i;const ps_control_page *p;
    if(!ps_panel_active(s)||s->editing)return 0;
    p=&s->panel->pages[s->tab];
    for(i=0;i<p->count;++i)if(p->controls[i].id==id){
        s->selected=i;s->control_header=0;
#if PS_CONTROL_SECTION_ROWS
        {unsigned start=i;while(start&&equal(p->controls[start-1].group,p->controls[i].group))--start;
        s->control_closed[s->tab][start/8]&=(ps_byte)~(1u<<(start%8));}
#endif
        s->dirty=1;return 1;
    }
    return 0;
}
PS_API int ps_panel_open(ps_session *s, unsigned page) {
    if (!s || ps_form_active(s) || !s->panel || page >= s->panel->count || !(s->io.flags & PS_INTERACTIVE))
        return 0;
    s->control_view = 1;
    s->dialog = 0;
    s->control_header = 0;
    table_open(s, page);
    s->notice[0] = 0;
    return 1;
}
PS_API void ps_panel_refresh(ps_session *s) {
    if (s)
        s->dirty = 1;
}

#if PS_WORKBENCH
#include "../host/workbench/controls.inc"
#endif
