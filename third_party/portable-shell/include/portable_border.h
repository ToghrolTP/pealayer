#ifndef PORTABLE_BORDER_H
#define PORTABLE_BORDER_H
/* Every glyph occupies one terminal cell. No emoji, variation selectors or
   font metrics enter layout/hit testing. Both C and C++ views share this set. */
#ifndef PS_BORDER_STYLES
#define PS_BORDER_STYLES 1
#endif
enum { PS_BORDER_ROUNDED, PS_BORDER_SINGLE, PS_BORDER_HEAVY_FRAME,
       PS_BORDER_HEAVY, PS_BORDER_DOUBLE, PS_BORDER_STYLE_COUNT };
#ifndef PS_BORDER_DEFAULT
#define PS_BORDER_DEFAULT PS_BORDER_ROUNDED
#endif
enum { PS_BORDER_TOP_LEFT, PS_BORDER_TOP, PS_BORDER_TOP_JOIN, PS_BORDER_TOP_RIGHT,
       PS_BORDER_OUTER_VERTICAL, PS_BORDER_INNER_VERTICAL,
       PS_BORDER_MIDDLE_LEFT, PS_BORDER_HORIZONTAL, PS_BORDER_CROSS, PS_BORDER_MIDDLE_RIGHT,
       PS_BORDER_BOTTOM_LEFT, PS_BORDER_BOTTOM, PS_BORDER_BOTTOM_JOIN, PS_BORDER_BOTTOM_RIGHT,
       PS_BORDER_JOIN_UP, PS_BORDER_JOIN_DOWN, PS_BORDER_PART_COUNT };
static const char *const ps_border_light_glyphs[]={"╭","─","┬","╮","│","│","├","─","┼","┤","╰","─","┴","╯","┴","┬"};
static inline int ps_border_style_valid(unsigned style){
    return style<PS_BORDER_STYLE_COUNT&&(PS_BORDER_STYLES||style==PS_BORDER_ROUNDED);
}
static inline const char *ps_border_glyph(unsigned style,int unicode,unsigned part){
    if(part>=PS_BORDER_PART_COUNT)return "";
    if(!unicode)return part==PS_BORDER_OUTER_VERTICAL||part==PS_BORDER_INNER_VERTICAL?"|":
        part==PS_BORDER_TOP||part==PS_BORDER_BOTTOM||part==PS_BORDER_HORIZONTAL?"-":"+";
#if PS_BORDER_STYLES
    {static const char *const single[]={"┌","─","┬","┐","│","│","├","─","┼","┤","└","─","┴","┘","┴","┬"};
     static const char *const mixed[]={"┏","━","┯","┓","┃","│","┠","─","┼","┨","┗","━","┷","┛","┴","┬"};
     static const char *const heavy[]={"┏","━","┳","┓","┃","┃","┣","━","╋","┫","┗","━","┻","┛","┻","┳"};
     static const char *const doubled[]={"╔","═","╦","╗","║","║","╠","═","╬","╣","╚","═","╩","╝","╩","╦"};
     if(style==PS_BORDER_SINGLE)return single[part];
     if(style==PS_BORDER_HEAVY_FRAME)return mixed[part];
     if(style==PS_BORDER_HEAVY)return heavy[part];
     if(style==PS_BORDER_DOUBLE)return doubled[part];}
#else
    (void)style;
#endif
    return ps_border_light_glyphs[part];
}
/* Adapt existing canonical light glyphs without creating a second renderer. */
static inline unsigned ps_border_light_part(const char *glyph){
    unsigned i,j;
    for(i=0;i<PS_BORDER_PART_COUNT;++i){for(j=0;glyph[j]&&glyph[j]==ps_border_light_glyphs[i][j];++j){}
        if(!glyph[j]&&!ps_border_light_glyphs[i][j])return i;}
    return PS_BORDER_PART_COUNT;
}
#endif
