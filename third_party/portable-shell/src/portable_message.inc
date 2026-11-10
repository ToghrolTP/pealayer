/* Synchronous delivery from the consumer's event loop; no worker or host I/O. */
enum { MESSAGE_UTF8_BYTES=4 };
static unsigned message_rune(const char *text,unsigned bytes,unsigned *count,char utf[MESSAGE_UTF8_BYTES+1]){
    unsigned i,n=min(bytes,MESSAGE_UTF8_BYTES),c;
    for(i=0;i<n;++i){utf[i]=text[i];}utf[n]=0;
    c=rune(utf,count);return c;
}
static void message_notice(ps_session *s,const char *text,unsigned bytes){
    unsigned i=0,n=0;
    while(i<bytes&&text[i]&&n+1<sizeof s->notice){
        char utf[MESSAGE_UTF8_BYTES+1];unsigned size,c;
        if((unsigned char)text[i]==27){i+=(unsigned)ps_escape_span(text+i,bytes-i);continue;}
        c=message_rune(text+i,bytes-i,&size,utf);i+=size;
        if(c=='\n'||c=='\r'){if(n)break;continue;}
        if(c<32||c==127||(c>=128&&c<160))continue;
        if(c==0xfffd||(!(s->io.flags&PS_UNICODE)&&c>126)){utf[0]='?';size=1;}
        if(n+size>=sizeof s->notice)break;
        copy(s->notice+n,utf,size);n+=size;
    }
    s->notice[n]=0;s->dirty=1;
    /* Keep each editor's existing save/validation error above an event notice. */
    if(s->document_open&&!s->document_notice)s->document_notice=s->notice;
    if(ps_hex_editor_active(s)&&!s->hex_notice)s->hex_notice=s->notice;
    if(s->file_count&&!s->file_notice)s->file_notice=s->notice;
#if PS_FORM
    if(s->form&&!s->form_error)s->form_error=s->notice;
#endif
    ps_resize(s,s->io.columns,s->io.rows);
}
static unsigned message_begin(ps_session *s){
    unsigned redraw=prompt_live(s);
    frame_begin(s);
    if(redraw)prompt_erase(s);
    return redraw;
}
static void message_end(ps_session *s,unsigned redraw){
    if(redraw)prompt(s);
    frame_end(s);
}
PS_API unsigned ps_message_block(ps_session *s,unsigned value,const char *text,unsigned bytes){
    unsigned i=0,column=0,ended=0,redraw,available;
    if(!s||!text||!bytes||value>PS_STYLE_ACCENT||s->exit)return 0;
    /* Find the bounded C-string extent before touching the display. */
    while(i<bytes&&text[i])++i;
    bytes=i;if(!bytes)return 0;
    if(message_canvas(s)){message_notice(s,text,bytes);return bytes;}
    redraw=message_begin(s);style(s,value);available=s->io.columns>1?s->io.columns-1:1;
    for(i=0;i<bytes;){
        char utf[MESSAGE_UTF8_BYTES+1];unsigned size,c,w;
        if((unsigned char)text[i]==27){i+=(unsigned)ps_escape_span(text+i,bytes-i);continue;}
        c=message_rune(text+i,bytes-i,&size,utf);i+=size;
        if(c=='\r'||c=='\n'){
            if(c=='\r'&&i<bytes&&text[i]=='\n')++i;
            ps_newline(s);column=0;ended=1;continue;
        }
        if(c<32||c==127||(c>=128&&c<160))continue;
        if(c==0xfffd||(!(s->io.flags&PS_UNICODE)&&c>126)){utf[0]='?';size=1;c='?';}
        w=(s->io.flags&PS_UNICODE)?width(c):1;
        if(column+w>available){ps_newline(s);column=0;}
        raw(s,utf,size);column+=w;ended=0;
    }
    reset(s);if(!ended)ps_newline(s);
    message_end(s,redraw);return bytes;
}
PS_API void ps_message(ps_session *s,unsigned value,const char *text){
    unsigned redraw;
    if(!s||!text||s->exit||value>PS_STYLE_ACCENT)return;
    if(*text){ps_message_block(s,value,text,length(text));return;}
    /* Preserve the existing empty-message blank line / clear-notice behavior. */
    if(message_canvas(s)){message_notice(s,text,0);return;}
    redraw=message_begin(s);reset(s);ps_newline(s);message_end(s,redraw);
}
