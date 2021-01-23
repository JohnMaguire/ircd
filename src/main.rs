use std::convert::TryFrom;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

mod config;
mod structs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // read config
    let config = config::get_config("./config.toml")?;

    // listen for connection on 127.0.0.1:6667
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 6667);
    let listener = TcpListener::bind(socket)?;
    println!("Listening on 127.0.0.1:6667");

    let (mut tcp_stream, addr) = listener.accept()?; // blocks until connection
    println!("Connection from {:?}", addr);

    let read_stream = tcp_stream.try_clone()?;
    let reader = BufReader::new(read_stream);
    let lines = reader.lines();

    for line in lines {
        // translate to internal irc message struct
        let line = line.unwrap();
        let irc_message = structs::IrcMessage::try_from(line.as_str())?;

        // decide whether to generate a reply
        let mut replies: Vec<structs::Reply> = vec![];
        match irc_message.to_command() {
            Ok(command) => {
                println!("{:?} -> {:?}", irc_message, command);

                match command {
                    structs::Command::USER(user, _mode, _unused, _realname) => {
                        replies.push(structs::Reply::RPL_WELCOME {
                            nick: "nick".to_owned(),
                            user: user.to_owned(),
                            host: "host".to_owned(),
                        });
                        replies.push(structs::Reply::RPL_YOURHOST {
                            nick: "nick".to_owned(),
                            server_name: config.irc.hostname.clone(),
                            version: "0.1.0".to_owned(),
                        });
                    }
                    _ => (),
                };
            }
            Err(error) => {
                println!("{:?} -> {:?}", irc_message, error);
                match error {
                    structs::ParseError::UnknownCommandError { command } => {
                        replies.push(structs::Reply::ERR_UNKNOWNCOMMAND { command })
                    }
                    structs::ParseError::MissingCommandParameterError {
                        command,
                        parameter: _,
                        index: _,
                    } => replies.push(structs::Reply::ERR_NEEDMOREPARAMS { command }),
                }
            }
        }

        for reply in replies {
            tcp_stream.write(reply.as_line().as_bytes())?;
        }
    }

    Ok(())
}
