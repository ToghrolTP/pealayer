/* Borderless reports share text flow and terminal primitives with command help. */
#if PS_REPORT
static int report_valid(const ps_report_row *rows,unsigned count){
    unsigned i,j;
    if(!rows&&count)return 0;
    for(i=0;i<count;++i){const char *texts[]={rows[i].label,rows[i].annotation,rows[i].value};
        for(j=0;j<PS_COUNT_OF(texts);++j){const char *p=texts[j];
            while(p&&*p){unsigned n,c=rune(p,&n);p+=n;
                if((c<32&&c!='\n'&&c!='\t')||c==127||(c>=128&&c<160))return 0;}
        }
    }
    return 1;
}
static unsigned report_value_role(unsigned flags){
    return flags&PS_REPORT_ERROR?PS_INK_ERROR:flags&PS_REPORT_WARNING?PS_INK_WARNING:
        flags&PS_REPORT_SUCCESS?PS_INK_SUCCESS:flags&PS_REPORT_INACTIVE?PS_INK_INACTIVE:
        flags&PS_REPORT_NUMBER?PS_INK_NUMBER:PS_INK_TEXT;
}
#endif
PS_API unsigned ps_print_report(ps_session *s,const ps_report_row *rows,unsigned count,unsigned style){
#if PS_REPORT
    unsigned i,available,column=0;
    if(!s||!report_valid(rows,count))return 0;
    available=flow_width(s);
    if(style&PS_REPORT_COLUMNS){
        for(i=0;i<count;++i)if(!(rows[i].flags&(PS_REPORT_SECTION|PS_REPORT_DIVIDER))){
            unsigned n=flow_cells(s,rows[i].label),note=flow_cells(s,rows[i].annotation);
            n+=note?note+1:0;if(n>column)column=n;
        }
        column=min(min(column,available/2)+4,available>10?available-4:available/2);
    }
    for(i=0;i<count;++i){
        const ps_report_row *r=rows+i;
        const char *label=r->label?r->label:"",*note=r->annotation?r->annotation:"",*value=r->value?r->value:"";
        unsigned l=flow_cells(s,label),n=flow_cells(s,note),v=flow_cells(s,value),left=l+(n?n+1:0),pad,j;
        if(r->flags&PS_REPORT_GAP)ps_newline(s);
        if(r->flags&PS_REPORT_DIVIDER){
            ink(s,PS_INK_BORDER);out(s,"  ");
            for(j=2;j<available;++j)out(s,(s->io.flags&PS_UNICODE)?"─":"-");
            reset(s);ps_newline(s);
            if(!left&&!v)continue;
        }
        if((style&PS_REPORT_COLUMNS)&&!(r->flags&PS_REPORT_SECTION)){
            out(s,"  ");ink(s,PS_INK_TEXT);
            if(left+4>column){flow_text(s,label,2,2);
                if(n){out(s,"    ");ink(s,PS_INK_MUTED);flow_text(s,note,4,4);}
                spaces(s,column);
            }else{ps_write(s,label);if(n){out(s," ");ink(s,PS_INK_MUTED);ps_write(s,note);}
                spaces(s,column-2-left);}
            ink(s,report_value_role(r->flags));flow_text(s,value,column,column);reset(s);continue;
        }
        if(left+v+6>available){
            ink(s,(r->flags&PS_REPORT_SECTION)?PS_INK_HEADING:PS_INK_TEXT);
            out(s,"  ");flow_text(s,label,2,2);reset(s);
            if(n){ink(s,PS_INK_MUTED);out(s,"    ");flow_text(s,note,4,4);reset(s);}
            if(v){ink(s,report_value_role(r->flags));
                if(v+2<=available){spaces(s,available-v);ps_write(s,value);ps_newline(s);}
                else {out(s,"    ");flow_text(s,value,4,4);}reset(s);}
            continue;
        }
        out(s,"  ");ink(s,(r->flags&PS_REPORT_SECTION)?PS_INK_HEADING:PS_INK_TEXT);
        if(r->flags&PS_REPORT_SECTION)ansi(s,"\033[1m");
        ps_write(s,label);reset(s);
        if(n){out(s," ");ink(s,PS_INK_MUTED);ps_write(s,note);}
        out(s," ");ink(s,PS_INK_MUTED);
        pad=available-2-left-1-(v?v+1:0);
        for(j=0;j<pad;++j)out(s,(style&PS_REPORT_SPACES)?" ":
            ((style&PS_REPORT_DENSE)||!(j&1))?".":" ");
        if(v){out(s," ");ink(s,report_value_role(r->flags));ps_write(s,value);}
        reset(s);ps_newline(s);
    }
    return count;
#else
    (void)s;(void)rows;(void)count;(void)style;return 0;
#endif
}
PS_API unsigned ps_message_report(ps_session *s,const ps_report_row *rows,unsigned count,unsigned style){
#if PS_REPORT
    unsigned redraw,rendered,i;
    if(!s||!count||s->exit||!report_valid(rows,count))return 0;
    if(message_canvas(s)){
        for(i=0;i<count;++i)if(rows[i].value&&*rows[i].value){
            char summary[sizeof s->notice];
            ps_terminal_format(summary,sizeof summary,"%s%s%s",rows[i].label?rows[i].label:"",
                rows[i].label&&*rows[i].label?": ":"",rows[i].value);
            message_notice(s,summary,length(summary));return 0;
        }
        for(i=0;i<count;++i)if(rows[i].label&&*rows[i].label){message_notice(s,rows[i].label,length(rows[i].label));break;}
        return 0;
    }
    redraw=message_begin(s);rendered=ps_print_report(s,rows,count,style);message_end(s,redraw);return rendered;
#else
    (void)s;(void)rows;(void)count;(void)style;return 0;
#endif
}
