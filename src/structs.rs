use std::convert::TryFrom;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct IrcMessage<'a> {
    pub prefix: Option<&'a str>,
    pub command: &'a str,
    pub command_parameters: Vec<&'a str>,
}

impl<'a> TryFrom<&'a str> for IrcMessage<'a> {
    type Error = String;

    /// Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use ircd::structs::IrcMessage;
    ///
    /// let s = ":irc.darkscience.net PRIVMSG Cardinal :this is a test";
    /// let irc_message = IrcMessage::try_from(s)?;
    ///
    /// assert_eq!(irc_message, IrcMessage {
    ///     prefix: Some("irc.darkscience.net"),
    ///     command: "PRIVMSG",
    ///     command_parameters: vec!["Cardinal", "this is a test"],
    /// });
    ///
    /// Ok::<(), String>(())
    /// ```
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        // It seems I need to set something like...
        //   fn try_from<'a>(s: &'a str) -> &'a Result<Self, Self::Error>
        // But I can't return a reference to the Result, so how can I set the
        // lifetime for the Result?
        if s == "" {
            return Err(Self::Error::from("IRC message may not be empty"));
        }

        let mut start = 0;
        let mut end = s.len();

        // check for optional prefix
        let prefix: Option<&str> = {
            match &s[start..end].find(':') {
                Some(0) => {
                    start += 1;
                    match &s[start..end].find(' ') {
                        None => {
                            // it's not clear if there is a prefix following the colon or not, but
                            // we can be sure that there is no command, which is required
                            return Err(Self::Error::from(
                                "Found prefix indication, but no command",
                            ));
                        }
                        Some(0) => {
                            // prefix colon may not precede a space
                            return Err(Self::Error::from(
                                "Found prefix indication, but no prefix",
                            ));
                        }
                        Some(prefix_end) => {
                            let prefix = &s[start..*prefix_end + 1];
                            start = *prefix_end + 2;
                            Some(prefix)
                        }
                    }
                }
                // must be a trailing parameter
                _ => None,
            }
        };

        // check for required command
        let command = {
            if let Some(idx) = &s[start..end].find(' ') {
                let command = &s[start..start + *idx];
                start += idx + 1;
                command
            } else {
                return Err(Self::Error::from("Missing required command"));
            }
        };

        // check for optional command parameters
        // there is a parameter beginning with a :, it is the last parameter, and everything
        // following the :, including spaces, should be included
        let trailer = {
            if let Some(idx) = &s[start..end].find(" :") {
                let trailer = &s[start + idx + 2..];
                end = start + *idx;
                Some(trailer)
            } else {
                None
            }
        };

        let mut command_parameters: Vec<&str> = s[start..end].split(" ").collect::<Vec<&str>>();

        // add trailer if there was one
        if trailer.is_some() {
            command_parameters.push(trailer.unwrap());
        }

        Ok(IrcMessage {
            prefix: prefix,
            command: command,
            command_parameters: command_parameters,
        })
    }
}

impl From<IrcMessage<'_>> for String {
    fn from(irc_message: IrcMessage) -> String {
        let mut message = "".to_owned();
        message.push_str(
            irc_message
                .prefix
                .map_or("".to_string(), |s| format!(":{} ", s))
                .as_str(),
        );
        message.push_str(format!("{} ", irc_message.command).as_str());
        message.push_str(irc_message.command_parameters.join(" ").as_str());
        message.push_str("\r\n");

        message
    }
}

impl fmt::Display for IrcMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match &self.prefix {
            Some(p) => format!("{}{} ", ":", p),
            _ => "".to_owned(),
        };

        let command_parameters = {
            let con = &self.command_parameters;
            match con.as_slice() {
                [] => "".to_owned(),
                _ => format!(" {}", con.join(" ")),
            }
        };

        write!(f, "{}{}{}", prefix, &self.command, command_parameters)
    }
}

#[allow(non_camel_case_types)]
pub enum Reply {
    RPL_WELCOME(String, String, String),
    RPL_YOURHOST(String, String, String),
    RPL_CREATED(String, String, String),
    RPL_MYINFO(String, String, String),
}

impl Reply {
    fn as_str(self: &Self) -> &str {
        match self {
            Reply::RPL_WELCOME(_, _, _) => "001",
            Reply::RPL_YOURHOST(_, _, _) => "002",
            Reply::RPL_CREATED(_, _, _) => "003",
            Reply::RPL_MYINFO(_, _, _) => "004",
        }
    }

    pub fn as_line(self: &Self) -> Result<String, String> {
        match self {
            Reply::RPL_WELCOME(nick, user, host) => Ok(String::from(IrcMessage {
                prefix: Some("localhost"),
                command: self.as_str(),
                command_parameters: vec![
                    &nick,
                    format!("Welcome to the network {}!{}@{}", nick, user, host).as_str(),
                ],
            })),
            _ => Err(String::from(format!("Reply is not implemented"))),
        }
    }
}

#[derive(Debug)]
pub enum Command<'a> {
    PASS(&'a str),
    NICK(&'a str),
    USER(&'a str, i8, &'a str, &'a str),
}

impl IrcMessage<'_> {
    pub fn to_command(&self) -> Result<Command, String> {
        match self.command {
            "PASS" => {
                let password = self
                    .command_parameters
                    .get(0)
                    .ok_or("PASS is missing a password parameter")?;
                Ok(Command::PASS(password.to_owned()))
            }
            "NICK" => {
                let nick = self
                    .command_parameters
                    .get(0)
                    .ok_or("NICK is missing a nick parameter")?;
                Ok(Command::NICK(nick.to_owned()))
            }
            "USER" => {
                let user = self
                    .command_parameters
                    .get(0)
                    .ok_or("USER is missing a user parameter")?;
                let mode = self
                    .command_parameters
                    .get(1)
                    .ok_or("USER is missing a mode parameter")?;
                let unused = self
                    .command_parameters
                    .get(2)
                    .ok_or("USER is missing a unused parameter")?;
                let realname = self
                    .command_parameters
                    .get(3)
                    .ok_or("USER is missing a realname parameter")?;
                Ok(Command::USER(
                    user.to_owned(),
                    mode.parse().or(Err(String::from("Invalid usermode")))?,
                    unused.to_owned(),
                    realname.to_owned(),
                ))
            }
            _ => Err(String::from("No command matched")),
        }
    }
}
