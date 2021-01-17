use std::convert::TryFrom;
use std::fmt;

type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug)]
pub enum ParseError {
    UnknownCommandError(UnknownCommand),
    MissingCommandParameterError(MissingCommandParameter),
}

#[derive(Debug)]
pub struct UnknownCommand {
    pub command: String,
}

#[derive(Debug)]
pub struct MissingCommandParameter {
    pub command: String,
    pub parameter: String,
    pub index: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            ParseError::UnknownCommandError(error) => format!("Unknown command: {}", error.command),
            ParseError::MissingCommandParameterError(error) => format!(
                "Command {} missing parameter: {}",
                error.command, error.parameter
            ),
        };
        write!(f, "{}", message)
    }
}

impl std::error::Error for ParseError {}

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
    fn try_from(s: &'a str) -> std::result::Result<Self, Self::Error> {
        if s == "" {
            return Err(Self::Error::from("IRC message may not be empty"));
        }

        let mut start = 0;

        // check for optional prefix
        let prefix: Option<&str> = {
            match s.find(':') {
                Some(0) => {
                    start += 1;
                    match &s[start..].find(' ') {
                        // prefix indicator must not be followed by a space, and a prefix must be
                        // followed by a command
                        None | Some(0) => {
                            return Err(Self::Error::from(
                                "Found prefix indication, followed by invalid prefix",
                            ))
                        }
                        Some(prefix_end) => {
                            let prefix = &s[start..*prefix_end + 1];
                            // skip over the space that follows the prefix as well
                            start += *prefix_end + 1;
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
            let idx = s[start..].find(' ').unwrap_or(s[start..].len());
            let command = &s[start..start + idx];
            // do not skip the space because detecting a trailer later will rely on the fact that a
            // trailng param colon must be prefixed by a space
            start += idx;

            command
        };

        // check for optional command parameters
        let command_parameters: Vec<&str> = {
            let mut end = s.len();

            // if there is a parameter beginning with a : it is the last parameter and everything
            // following the : should be included
            let trailer = {
                if let Some(idx) = &s[start..].find(" :") {
                    let trailer = &s[start + idx + 2..];
                    end = start + *idx;
                    Some(trailer)
                } else {
                    None
                }
            };

            let mut command_parameters: Vec<&str> = if start < end {
                // skip over the leftover space that follows the command
                start += 1;
                s[start..end].split(" ").collect()
            } else {
                vec![]
            };

            // add trailer if there was one
            if trailer.is_some() {
                command_parameters.push(trailer.unwrap());
            }

            command_parameters
        };

        Ok(IrcMessage {
            prefix: prefix,
            command: command,
            command_parameters: command_parameters,
        })
    }
}

#[allow(non_camel_case_types)]
pub enum Reply {
    RPL_WELCOME(String, String, String),
    // RPL_YOURHOST(String, String, String),
    // RPL_CREATED(String, String, String),
    // RPL_MYINFO(String, String, String),
    ERR_UNKNOWNCOMMAND(String),
    ERR_NEEDMOREPARAMS(String),
}

impl Reply {
    fn as_str(self: &Self) -> &str {
        match self {
            Reply::RPL_WELCOME(_, _, _) => "001",
            // Reply::RPL_YOURHOST(_, _, _) => "002",
            // Reply::RPL_CREATED(_, _, _) => "003",
            // Reply::RPL_MYINFO(_, _, _) => "004",
            Reply::ERR_UNKNOWNCOMMAND(_) => "421",
            Reply::ERR_NEEDMOREPARAMS(_) => "461",
        }
    }

    pub fn as_line(self: &Self) -> String {
        match self {
            // Command responses
            Reply::RPL_WELCOME(nick, user, host) => IrcMessage {
                prefix: Some("localhost"),
                command: self.as_str(),
                command_parameters: vec![
                    &nick,
                    format!("Welcome to the network {}!{}@{}", nick, user, host).as_str(),
                ],
            }
            .to_line(),

            // Error replies
            Reply::ERR_UNKNOWNCOMMAND(command) => IrcMessage {
                prefix: Some("localhost"),
                command: self.as_str(),
                command_parameters: vec![command, "Unknown command"],
            }
            .to_line(),

            // Error replies
            Reply::ERR_NEEDMOREPARAMS(command) => IrcMessage {
                prefix: Some("localhost"),
                command: self.as_str(),
                command_parameters: vec![command, "Not enough parameters"],
            }
            .to_line(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command<'a> {
    PASS(&'a str),
    NICK(&'a str),
    USER(&'a str, &'a str, &'a str, &'a str),
}

impl IrcMessage<'_> {
    /// Examples
    ///
    /// ```
    /// use ircd::structs::{Command, IrcMessage};
    ///
    /// let irc_message = IrcMessage{
    ///     prefix: None,
    ///     command: "USER",
    ///     command_parameters: vec!["Cardinal", "8", "*", "Cardinal"],
    /// };
    /// let command = irc_message.to_command().unwrap();
    ///
    /// assert_eq!(command, Command::USER("Cardinal", "8", "*", "Cardinal"));
    ///
    /// Ok::<(), String>(())
    /// ```
    pub fn to_command(&self) -> Result<Command> {
        match self.command {
            "PASS" => {
                let password = self.get_command_parameter(0, "password")?;
                Ok(Command::PASS(password))
            }
            "NICK" => {
                let nick = self.get_command_parameter(0, "nick")?;
                Ok(Command::NICK(nick))
            }
            "USER" => {
                let user = self.get_command_parameter(0, "user")?;
                let mode = self.get_command_parameter(1, "mode")?;
                let unused = self.get_command_parameter(2, "unused")?;
                let realname = self.get_command_parameter(3, "realname")?;
                Ok(Command::USER(user, mode, unused, realname))
            }
            _ => Err(ParseError::UnknownCommandError(UnknownCommand {
                command: self.command.to_owned(),
            })),
        }
    }

    fn get_command_parameter(&self, idx: usize, name: &str) -> Result<&str> {
        let param = self.command_parameters.get(idx).ok_or_else(|| {
            ParseError::MissingCommandParameterError(MissingCommandParameter {
                command: self.command.to_owned(),
                parameter: name.to_owned(),
                index: idx,
            })
        })?;

        Ok(param)
    }

    /// Examples
    ///
    /// ```
    /// use ircd::structs::{Command, IrcMessage};
    ///
    /// let irc_message = IrcMessage{
    ///     prefix: Some("localhost"),
    ///     command: "PRIVMSG",
    ///     command_parameters: vec!["Cardinal", "this is an example"],
    /// };
    /// let s = irc_message.to_line();
    ///
    /// assert_eq!(s, ":localhost PRIVMSG Cardinal :this is an example\r\n".to_owned());
    ///
    /// Ok::<(), String>(())
    /// ```
    ///
    /// Note: The last parameter will always be prefixed with a colon.
    pub fn to_line(mut self) -> String {
        let mut message = "".to_owned();
        message.push_str(
            self.prefix
                .map_or("".to_string(), |s| format!(":{} ", s))
                .as_str(),
        );
        message.push_str(self.command);

        if self.command_parameters.len() > 0 {
            message.push_str(" ");

            // a little dance to stick the last param behind a colon to ensure that params with
            // spaces work correctly (e.g. messages)
            let mut params = self
                .command_parameters
                .drain(0..self.command_parameters.len() - 1)
                .collect::<Vec<&str>>();
            let last_param = format!(":{}", self.command_parameters.pop().unwrap());
            params.push(last_param.as_str());

            message.push_str(params.join(" ").as_str());
        }

        message.push_str("\r\n");

        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_parameters_not_required() -> std::result::Result<(), String> {
        let s = "LIST";
        let irc_message = IrcMessage::try_from(s)?;

        assert_eq!(
            irc_message,
            IrcMessage {
                prefix: None,
                command: "LIST",
                command_parameters: vec![],
            }
        );

        Ok(())
    }

    #[test]
    fn command_prefix() -> std::result::Result<(), String> {
        let s = ":irc.darkscience.net LIST";
        let irc_message = IrcMessage::try_from(s)?;

        assert_eq!(
            irc_message,
            IrcMessage {
                prefix: Some("irc.darkscience.net"),
                command: "LIST",
                command_parameters: vec![],
            }
        );

        Ok(())
    }

    #[test]
    fn command_parameters() -> std::result::Result<(), String> {
        let s = "PRIVMSG Cardinal :this is a test";
        let irc_message = IrcMessage::try_from(s)?;

        assert_eq!(
            irc_message,
            IrcMessage {
                prefix: None,
                command: "PRIVMSG",
                command_parameters: vec!["Cardinal", "this is a test"],
            }
        );

        Ok(())
    }

    #[test]
    fn command_parameters_no_trailer() -> std::result::Result<(), String> {
        let s = "MODE #test +v Cardinal";
        let irc_message = IrcMessage::try_from(s)?;

        assert_eq!(
            irc_message,
            IrcMessage {
                prefix: None,
                command: "MODE",
                command_parameters: vec!["#test", "+v", "Cardinal"],
            }
        );

        Ok(())
    }

    #[test]
    fn command_parameter_trailer_only() -> std::result::Result<(), String> {
        let s = "PONG :irc.darkscience.net";
        let irc_message = IrcMessage::try_from(s)?;

        assert_eq!(
            irc_message,
            IrcMessage {
                prefix: None,
                command: "PONG",
                command_parameters: vec!["irc.darkscience.net"],
            }
        );

        Ok(())
    }
}
