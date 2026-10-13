grammar LOLCODEMarkdown;

lolcode
    : HAI documentContent KBYE EOF
    ;

documentContent
    : variableDefinition body
    | comment afterLeadingComment
    | head body
    | nonCommentBodyElement body
    |
    ;

afterLeadingComment
    : comment afterLeadingComment
    | head body
    | nonCommentBodyElement body
    |
    ;

commentSequence
    : comment*
    ;

head
    : MAEK_HEAD comment* (title comment*)? MKAY
    ;

title
    : GIMMEH_TITLE plainText OIC
    ;

body
    : bodyElement*
    ;

bodyElement
    : comment
    | nonCommentBodyElement
    ;

nonCommentBodyElement
    : paragraph
    | bold
    | list
    | newlineAnnotation
    | link
    | variableUse
    | textAtom
    ;

paragraph
    : MAEK_PARAGRAF optionalVariableDefinition paragraphElement* MKAY
    ;

optionalVariableDefinition
    : variableDefinition?
    ;

paragraphElement
    : comment
    | bold
    | list
    | newlineAnnotation
    | link
    | variableUse
    | textAtom
    ;

bold
    : GIMMEH_BOLD (variableUse | textAtom)* OIC
    ;

list
    : MAEK_LIST item (item | comment)* MKAY
    ;

item
    : GIMMEH_ITEM optionalVariableDefinition itemElement* OIC
    ;

itemElement
    : comment
    | bold
    | variableUse
    | textAtom
    ;

comment
    : OBTW textAtom* TLDR
    ;

newlineAnnotation
    : NEWLINE_TAG
    ;

link
    : GIMMEH_LINX plainText OIC
    ;

variableDefinition
    : IHAZ WORD ITIZ plainText MKAY
    ;

variableUse
    : GIMMEH_VAR WORD OIC
    ;

plainText
    : textAtom+
    ;

textAtom
    : WORD
    | PUNCTUATION
    ;

MAEK_PARAGRAF : '#' M A E K ' '+ P A R A G R A F;
GIMMEH_TITLE  : '#' G I M M E H ' '+ T I T L E;
GIMMEH_BOLD   : '#' G I M M E H ' '+ B O L D;
GIMMEH_ITEM   : '#' G I M M E H ' '+ I T E M;
GIMMEH_LINX   : '#' G I M M E H ' '+ L I N X;
GIMMEH_VAR    : '#' G I M M E H ' '+ V A R;
MAEK_HEAD     : '#' M A E K ' '+ H E A D;
MAEK_LIST     : '#' M A E K ' '+ L I S T;
NEWLINE_TAG   : '#' N E W L I N E;
HAI           : '#' H A I;
KBYE          : '#' K B Y E;
OBTW          : '#' O B T W;
TLDR          : '#' T L D R;
MKAY          : '#' M K A Y;
OIC           : '#' O I C;
IHAZ          : '#' I H A Z;
ITIZ          : '#' I T I Z;

WORD          : ('A'..'Z' | 'a'..'z' | '0'..'9')+;
PUNCTUATION   : (',' | '.' | '"' | ':' | '?' | '!' | '%' | '=' | '/')+;
WS            : (' ' | '\t' | '\r' | '\n')+ {$channel=HIDDEN;};

INVALID       : .;

fragment A : 'A' | 'a';
fragment B : 'B' | 'b';
fragment C : 'C' | 'c';
fragment D : 'D' | 'd';
fragment E : 'E' | 'e';
fragment F : 'F' | 'f';
fragment G : 'G' | 'g';
fragment H : 'H' | 'h';
fragment I : 'I' | 'i';
fragment J : 'J' | 'j';
fragment K : 'K' | 'k';
fragment L : 'L' | 'l';
fragment M : 'M' | 'm';
fragment N : 'N' | 'n';
fragment O : 'O' | 'o';
fragment P : 'P' | 'p';
fragment Q : 'Q' | 'q';
fragment R : 'R' | 'r';
fragment S : 'S' | 's';
fragment T : 'T' | 't';
fragment U : 'U' | 'u';
fragment V : 'V' | 'v';
fragment W : 'W' | 'w';
fragment X : 'X' | 'x';
fragment Y : 'Y' | 'y';
fragment Z : 'Z' | 'z';
