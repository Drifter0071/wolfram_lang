use logos::Logos;

// ==========================================
// 1. LEXER
// ==========================================
#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("local")]
    Local,
    #[token("function")]
    Function,
    #[token("return")]
    Return,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("class")]
    Class,
    #[token("public")]
    Public,
    #[token("private")]
    Private,
    #[token("self")]
    SelfKw,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("nil")]
    Nil,
    #[token("enum")]
    EnumKw,
    #[token("struct")]
    StructKw,
    #[token("import")]
    Import,
    #[token("as")]
    As,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("not")]
    Not,
    #[token("elif")]
    Elif,
    #[token("try")]
    Try,
    #[token("catch")]
    Catch,
    #[token("finally")]
    Finally,
    #[token("async")]
    Async,
    #[token("await")]
    Await,
    #[token("global")]
    Global,
    #[token("is")]
    Is,

    // Exponent
    #[token("^")]
    Caret,

    // Multi-char operators (must precede single-char equivalents)
    #[token("**")]
    StarStar,
    #[token("..")]
    DotDot,
    #[token("==")]
    EqEq,
    #[token("~=")]
    #[token("!=")]
    NotEq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("+=")]
    PlusAssign,
    #[token("-=")]
    MinusAssign,
    #[token("*=")]
    StarAssign,
    #[token("/=")]
    SlashAssign,
    #[token("%=")]
    PercentAssign,
    #[token("->")]
    Arrow,

    // Single-char operators and punctuation
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,
    #[token("=")]
    Assign,
    #[token(";")]
    Semicolon,
    #[token("?")]
    Question,
    #[token("@")]
    At,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().parse().ok())]
    Number(f64),

    #[regex(r#"f"([^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    FString(String),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    StringLit(String),

    #[regex(r"--[^\n]*", |lex| lex.slice().to_string(), allow_greedy = true)]
    CommentDash(String),

    #[regex(r"//[^\n]*", |lex| lex.slice().to_string(), allow_greedy = true)]
    CommentSlash(String),

    #[regex(r"[ \t\n\f]+", logos::skip)]
    Whitespace,
}
