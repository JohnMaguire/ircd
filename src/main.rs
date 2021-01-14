use std::convert::TryFrom;
use std::fmt;
use std::io::{BufRead, BufReader};
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
        let mut vec = s.trim_end_matches("\r\n").split(" ").collect::<Vec<&str>>();

        // check for a prefix in the message
        let maybe_prefix: &str = vec.first().unwrap();
        let prefix: Option<String> = match maybe_prefix.chars().next().unwrap() {
            ':' => Some(maybe_prefix.to_owned()),
            _ => None,
        };

        if prefix.is_some() {
            vec.drain(..1);
        }

        // command is required
        let command = vec.first().get_or_insert(&"").to_string();
        vec.drain(..1);

        // any remaining parameters are command parameters
        // FIXME: this should look for trailing and separate it into its own parameter
        let command_parameters: Vec<String> = vec.into_iter().map(|s| s.to_owned()).collect();

        Ok(IrcMessage {
            prefix: prefix,
            command: command,
            command_parameters: command_parameters,
        })
    }
}

impl fmt::Display for IrcMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match &self.prefix {
            Some(p) => format!("{}{} ", ":", p),
            _ => "".to_owned(),
        };

        let con = &self.command_parameters;
        let command_parameters = match con.as_slice() {
            [] => "".to_owned(),
            _ => format!(" {}", con.join(" ")),
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

impl From<&Reply> for i16 {
    fn from(reply: &Reply) -> i16 {
        match reply {
            Reply::RPL_WELCOME(_, _, _) => 001,
            Reply::RPL_YOURHOST(_, _, _) => 002,
            Reply::RPL_CREATED(_, _, _) => 003,
            Reply::RPL_MYINFO(_, _, _) => 004,
        }
    }
}

impl Reply {
    fn to_irc_message(&self) -> Result<IrcMessage, String> {
        match self {
            Reply::RPL_WELCOME(nick, user, host) => Ok(IrcMessage {
                prefix: Some("localhost".to_owned()),
                command: format!("{}", i16::from(self)),
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
                    .ok_or("NICK is missing a password parameter")?;
                Ok(Command::NICK(nick.to_owned()))
            }
            _ => Err(String::from("No command matched")),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // test messages to verify structs are working
    let message = IrcMessage {
        prefix: Some("irc.darkscience.net".to_owned()),
        command: "PRIVMSG".to_owned(),
        command_parameters: vec!["whoami".to_owned(), ":Welcome".to_owned()],
    };
    println!("{}", message);

    let welcome = Reply::RPL_WELCOME(
        "whoami".to_owned(),
        "whoami".to_owned(),
        "johnmaguire.me".to_owned(),
    );
    println!("{}", welcome.to_irc_message().unwrap());

    // ===================================================================== //

    // listen for connection
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 6667);
    let listener = TcpListener::bind(socket)?;
    println!("Listening on 127.0.0.1:6667");

    let (tcp_stream, addr) = listener.accept()?; // blocks until connection
    println!("Connection from {:?}", addr);

    // read input
    let mut input = String::new();
    let mut reader = BufReader::new(tcp_stream);
    let _ = reader.read_line(&mut input);
    println!("{:?} says: {}", addr, input);

    // translate to internal irc message struct
    let irc_message = IrcMessage::try_from(input)?;
    let command = irc_message.to_command().unwrap();
    println!("{:?} -> {:?}", irc_message, command);

    Ok(())
}
