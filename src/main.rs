use std::convert::TryFrom;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

mod structs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // listen for connection on 127.0.0.1:6667
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 6667);
    let listener = TcpListener::bind(socket)?;
    println!("Listening on 127.0.0.1:6667");

    let (mut tcp_stream, addr) = listener.accept()?; // blocks until connection
    println!("Connection from {:?}", addr);

    // read input
    // let mut input = String::new();
    // let _ = reader.read_line(&mut input);
    let read_stream = tcp_stream.try_clone()?;
    let reader = BufReader::new(read_stream);
    let lines = reader.lines();

    for line in lines {
        let line = line.unwrap();
        println!("{:?} says: {}", addr, line);

        // translate to internal irc message struct
        let irc_message = structs::IrcMessage::try_from(line)?;
        let command = irc_message.to_command().unwrap();
        println!("{:?} -> {:?}", irc_message, command);

        if let Ok(command) = irc_message.to_command() {
            match command {
                structs::Command::USER(user, _mode, _unused, _realname) => {
                    // send a welcome message
                    let reply = structs::Reply::RPL_WELCOME(
                        "nick".to_owned(),
                        user.to_owned(),
                        "ident".to_owned(),
                    )
                    .to_irc_message()
                    .unwrap();
                    tcp_stream.write(String::from(reply).as_bytes())?;
                }
                _ => {
                    println!("Not handling command");
                }
            }
        }
    }

    Ok(())
}
