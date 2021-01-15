use std::convert::TryFrom;
use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

#[derive(Debug)]
struct IrcMessage {
    prefix: Option<String>,
    command: String,
    command_parameters: Vec<String>,
}

impl TryFrom<String> for IrcMessage {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s == "" {
            return Err(Self::Error::from("IRC message may not be empty"));
        }

        let mut s = s.trim_end_matches("\r\n").to_owned();

        // check for optional prefix
        let mut prefix: Option<String> = None;
        if let Some(idx) = s.find(':') {
            prefix = match idx {
                0 => {
                    s.remove(idx);
                    match s.find(' ') {
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
                            s.remove(prefix_end);
                            Some(s.drain(..prefix_end).collect::<String>())
                        }
                    }
                }
                // must be a trailing parameter
                _ => None,
            };
        }

        // check for required command
        let command = {
            if let Some(idx) = s.find(' ') {
                s.remove(idx);
                s.drain(..idx).collect::<String>()
            } else {
                return Err(Self::Error::from("Missing required command"));
            }
        };

        // check for optional command parameters
        // there is a parameter beginning with a :, it is the last parameter, and everything
        // following the :, including spaces, should be included
        let trailer = {
            if let Some(idx) = s.find(" :") {
                s.drain(idx..idx + 2);
                Some(s.drain(idx..).collect::<String>())
            } else {
                None
            }
        };

        let mut command_parameters: Vec<String> = s
            .split(" ")
            .collect::<Vec<&str>>()
            .into_iter()
            .map(|s| s.to_owned())
            .collect();

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

impl From<IrcMessage> for String {
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

impl fmt::Display for IrcMessage {
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

impl From<&Reply> for String {
    fn from(reply: &Reply) -> String {
        match reply {
            Reply::RPL_WELCOME(_, _, _) => "001".to_owned(),
            Reply::RPL_YOURHOST(_, _, _) => "002".to_owned(),
            Reply::RPL_CREATED(_, _, _) => "003".to_owned(),
            Reply::RPL_MYINFO(_, _, _) => "004".to_owned(),
        }
    }
}

impl Reply {
    fn to_irc_message(&self) -> Result<IrcMessage, String> {
        match self {
            Reply::RPL_WELCOME(nick, user, host) => Ok(IrcMessage {
                prefix: Some("localhost".to_owned()),
                command: format!("{}", String::from(self)),
                command_parameters: vec![
                    nick.to_owned(),
                    format!("Welcome to the network {}!{}@{}", nick, user, host),
                ],
            }),
            _ => Err(String::from(format!("Reply is not implemented"))),
        }
    }
}

#[derive(Debug)]
enum Command {
    PASS(String),
    NICK(String),
    USER(String, i8, String, String),
}

impl IrcMessage {
    fn to_command(&self) -> Result<Command, String> {
        match self.command.as_str() {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // listen for connection
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 6667);
    let listener = TcpListener::bind(socket)?;
    println!("Listening on 127.0.0.1:6667");

    let (mut tcp_stream, addr) = listener.accept()?; // blocks until connection
    println!("Connection from {:?}", addr);

    // read input
    let mut input = String::new();
    let mut reader = BufReader::new(&tcp_stream);
    let _ = reader.read_line(&mut input);
    println!("{:?} says: {}", addr, input);

    // translate to internal irc message struct
    let irc_message = IrcMessage::try_from(input)?;
    let command = irc_message.to_command().unwrap();
    println!("{:?} -> {:?}", irc_message, command);

    // send a welcome message
    let reply = Reply::RPL_WELCOME("nick".to_owned(), "user".to_owned(), "ident".to_owned())
        .to_irc_message()
        .unwrap();
    tcp_stream.write(String::from(reply).as_bytes())?;

    Ok(())
}
