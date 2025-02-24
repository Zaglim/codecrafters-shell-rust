use my_derives::{FromInnerType, MayStartWith, MyFromStrParse, ZDisplay};

use strum::{EnumIter, EnumString, IntoStaticStr};

#[derive(Debug, Clone, FromInnerType, MyFromStrParse)]
pub enum OperatorChar {
    ControlCharacter(ControlCharacter),
    RedirectCharacter(RedirectCharacter),
}

#[derive(Debug, Clone, Copy, IntoStaticStr, EnumString, MayStartWith, ZDisplay)]
pub enum RedirectCharacter {
    #[strum(serialize = "<")]
    LessThan,
    #[strum(serialize = ">", serialize = "1>")]
    GreaterThan,
}

/// Seperates words. Some are delimiters, others should be retained as tokens
#[derive(Debug, Clone, FromInnerType, MyFromStrParse)]
pub enum Metacharacter {
    OperatorCharacter(OperatorChar),
    Blank(Blank),
}

/// Not important. Can be deleted once solved the issue
#[derive(Debug, Clone, IntoStaticStr, EnumIter, MyFromStrParse, ZDisplay)]
pub enum Blank {
    #[strum(serialize = " ")]
    Space,
    #[strum(serialize = "\t")]
    Tab,
}

#[derive(Debug, Clone, Copy, EnumIter, IntoStaticStr, MyFromStrParse, MayStartWith, ZDisplay)]
pub enum ControlCharacter {
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
}

#[derive(Debug, Clone, Copy, FromInnerType, MyFromStrParse, MayStartWith, ZDisplay)]
pub enum ControlOperator {
    ControlCharacter(ControlCharacter),
    ControlLong(ControlLong),
}

#[derive(Debug, Clone, Copy, IntoStaticStr, EnumIter, MyFromStrParse, MayStartWith, ZDisplay)]
pub enum ControlLong {
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
            Token::Operator(Operator::Control(ControlOperator::ControlCharacter(
                ControlCharacter::Semicolon | ControlCharacter::Newline
            )))
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

#[derive(Debug, Clone, Copy, FromInnerType, MyFromStrParse, MayStartWith, ZDisplay)]
pub enum RedirectOperator {
    SingleChar(RedirectCharacter),
    // todo more
}
