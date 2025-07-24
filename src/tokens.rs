use my_derives::{FromInnerType, MayStartWith, MyFromStrParse, ZDisplay};
use std::borrow::Borrow;
use std::ffi::OsStr;
use std::fmt::Display;
use std::path::PathBuf;
use strum::{AsRefStr, EnumString, IntoStaticStr};

#[derive(Debug, Clone, Copy, IntoStaticStr, EnumString, MayStartWith, ZDisplay, AsRefStr)]
pub enum RedirectOperator {
    #[strum(serialize = "<")]
    RStdin,
    #[strum(serialize = ">", serialize = "1>")]
    RStdout,
    #[strum(serialize = "2>")]
    RStderr,
    #[strum(serialize = ">>", serialize = "1>>")]
    AppendStdout,
    #[strum(serialize = "2>>")]
    AppendStderr,
}

impl RedirectOperator {
    #[inline]
    // passing by value because it is cheap and Self implements copy
    /// true iff `AppendStdout | AppendStderr`
    pub const fn appends(self) -> bool {
        matches!(self, Self::AppendStdout | Self::AppendStderr)
    }
}

/// pure delimiters while outside of token
#[inline]
pub fn is_shell_blank(c: impl Borrow<char>) -> bool {
    [' ', '\t'].contains(c.borrow())
}

#[derive(Debug, Clone, Copy, MyFromStrParse, MayStartWith, IntoStaticStr, ZDisplay, AsRefStr)]
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
#[derive(Debug, Clone, ZDisplay)]
pub enum Token {
    Word(Word),
    Operator(Operator),
}

impl AsRef<str> for Token {
    fn as_ref(&self) -> &str {
        match self {
            Self::Word(w) => w.as_ref(),
            Self::Operator(o) => o.as_ref(),
        }
    }
}

impl TryFrom<Token> for PathBuf {
    type Error = (); // todo

    /// Checks that the token does not have a special meaning
    ///
    /// # Examples
    ///
    /// ```
    /// let token = Token::from("if");
    /// assert!(PathBuf::try_from(token).is_err());
    ///
    /// let token = Token::from("some_something");
    /// assert!(PathBuf::try_from(token).is_ok());
    ///
    /// let token = Token::from("&&");
    /// assert!(PathBuf::from(token).is_err());
    /// ```
    fn try_from(token: Token) -> Result<Self, Self::Error> {
        match token {
            Token::Word(Word::SimpleWord(word)) => Ok(word.into()),
            _reserved => Err(()),
        }
    }
}

impl From<String> for Token {
    fn from(s: String) -> Self {
        if let Ok(operator) = s.parse::<Operator>() {
            return Self::Operator(operator);
        }
        Self::Word(s.into())
    }
}

impl AsRef<OsStr> for Token {
    fn as_ref(&self) -> &OsStr {
        <Self as AsRef<str>>::as_ref(self).as_ref()
    }
}

impl Token {
    pub const fn is_command_delimiter(&self) -> bool {
        use ControlOperator as CO;
        use Operator::Control;
        matches!(
            self,
            Self::Operator(Control(CO::Semicolon | CO::Newline | CO::And))
        )
    }
    pub const fn is_control_operator(&self) -> bool {
        matches!(self, Self::Operator(Operator::Control(_)))
    }

    pub const fn is_redirect_operator(&self) -> bool {
        matches!(self, Self::Operator(Operator::Redirect(_)))
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

impl AsRef<OsStr> for Word {
    fn as_ref(&self) -> &OsStr {
        <Self as AsRef<str>>::as_ref(self).as_ref()
    }
}

impl AsRef<str> for Word {
    fn as_ref(&self) -> &str {
        match self {
            Self::ReservedWord(r) => r.as_ref(),
            Self::SimpleWord(s) => s,
        }
    }
}

impl From<String> for Word {
    fn from(value: String) -> Self {
        match value.parse() {
            Ok(rw) => Self::ReservedWord(rw),
            Err(()) => Self::SimpleWord(value),
        }
    }
}

impl From<&str> for Word {
    fn from(value: &str) -> Self {
        match value.parse() {
            Ok(rw) => Self::ReservedWord(rw),
            Err(()) => Self::SimpleWord(value.to_string()),
        }
    }
}

impl Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::ReservedWord(reserved_word) => <&str>::from(reserved_word).to_string(),
                Self::SimpleWord(w) => w.to_owned(),
            }
        )
    }
}

#[derive(Debug, Clone, MyFromStrParse, IntoStaticStr, AsRefStr)]
pub enum ReservedWord {
    #[strum(serialize = "if")]
    If,
    #[strum(serialize = "then")]
    Then,
    #[strum(serialize = "time")]
    Time,
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

impl AsRef<str> for Operator {
    fn as_ref(&self) -> &str {
        match self {
            Self::Control(c) => c.as_ref(),
            Self::Redirect(r) => r.as_ref(),
        }
    }
}

impl Operator {
    pub fn may_start_with(value: &str) -> bool {
        ControlOperator::may_start_with(value) || RedirectOperator::may_start_with(value)
    }
}
