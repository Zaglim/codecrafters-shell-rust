use std::borrow::Borrow;

use my_derives::{FromInnerType, MayStartWith, MyFromStrParse, ZDisplay};

use strum::{EnumString, IntoStaticStr};
type CO = ControlOperator;

#[derive(Debug, Clone, Copy, IntoStaticStr, EnumString, MayStartWith, ZDisplay)]
pub enum RedirectOperator {
    #[strum(serialize = "<")]
    RStdin,
    #[strum(serialize = ">", serialize = "1>")]
    RStdout,
    #[strum(serialize = "2>")]
    RStderr,
}

/// pure delimiters while outside of token
#[inline]
pub fn is_blank(c: impl Borrow<char>) -> bool {
    [' ', '\t'].contains(c.borrow())
}

#[derive(Debug, Clone, Copy, MyFromStrParse, MayStartWith, IntoStaticStr, ZDisplay)]
pub enum ControlOperator {
    #[strum(serialize = "\n")]
    Newline,
    #[strum(serialize = "&")]
    Ampersand,
    #[strum(serialize = ";")]
    Semicolon,
    #[strum(serialize = "|")]
    Pipe,
    #[strum(serialize = "(")]
    OpenBracket,
    #[strum(serialize = ")")]
    CloseBracket,
    #[strum(serialize = "||")]
    Or,
    #[strum(serialize = "&&")]
    And,
    #[strum(serialize = ";;")]
    DoubleSemi,
    #[strum(serialize = ";&")]
    SemiAmp,
    #[strum(serialize = ";;&")]
    DoubleSemiAmp,
    #[strum(serialize = "|&")]
    PipeAmp,
}

/// "A sequence of characters considered a single unit by the shell. It is either a word or an operator."
///
/// (excludes Blanks)
///
/// -- [ref manual](https://www.gnu.org/software/bash/manual/bash.html#index-token)
#[derive(Debug, Clone, ZDisplay, FromInnerType)]
pub enum Token {
    Word(Word),
    Operator(Operator),
}

impl<T: AsRef<str>> From<T> for Token {
    fn from(value: T) -> Self {
        let s: &str = value.as_ref();

        if let Ok(operator) = s.parse::<Operator>() {
            return Self::Operator(operator);
        }
        Self::Word(s.into())
    }
}

impl Token {
    pub fn is_command_delimiter(&self) -> bool {
        matches!(
            self,
            Token::Operator(Operator::Control(CO::Semicolon | CO::Newline | CO::And))
        )
    }
}

/// "A sequence of characters treated as a unit by the shell. Words may not include unquoted metacharacters."
///
/// -- [ref manual](https://www.gnu.org/software/bash/manual/bash.html#index-word)
#[derive(Debug, Clone)]
pub enum Word {
    /// "A word that has a special meaning to the shell. Most reserved words introduce shell flow control constructs, such as for and while."
    ///
    /// -- [ref](https://www.gnu.org/software/bash/manual/bash.html#index-reserved-word)
    ReservedWord(ReservedWord),
    SimpleWord(String),
}

impl From<Word> for &str {
    fn from(_value: Word) -> Self {
        todo!()
    }
}

impl From<&str> for Word {
    fn from(value: &str) -> Self {
        match value.parse() {
            Ok(rw) => Self::ReservedWord(rw),
            Err(_) => Self::SimpleWord(value.to_string()),
        }
    }
}

impl std::fmt::Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Word::ReservedWord(reserved_word) => <&str>::from(reserved_word).to_string(),
                Word::SimpleWord(w) => w.to_owned(),
            }
        )
    }
}

#[derive(Debug, Clone, MyFromStrParse, IntoStaticStr)]
pub enum ReservedWord {
    #[strum(serialize = "if")]
    If,
    #[strum(serialize = "then")]
    Then,
    // todo
}

/// "A control operator or a redirection operator."
///
/// --[ref](https://www.gnu.org/software/bash/manual/bash.html#index-operator_002c-shell)
#[derive(Debug, Clone, Copy, FromInnerType, MyFromStrParse, ZDisplay)]
pub enum Operator {
    Control(ControlOperator),
    Redirect(RedirectOperator),
}

impl Operator {
    pub fn may_start_with(value: &str) -> bool {
        ControlOperator::may_start_with(value) || RedirectOperator::may_start_with(value)
    }
}
