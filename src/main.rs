use std::convert::TryFrom;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

mod structs;

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
    let irc_message = structs::IrcMessage::try_from(input)?;
    let command = irc_message.to_command().unwrap();
    println!("{:?} -> {:?}", irc_message, command);

    // send a welcome message
    let reply =
        structs::Reply::RPL_WELCOME("nick".to_owned(), "user".to_owned(), "ident".to_owned())
            .to_irc_message()
            .unwrap();
    tcp_stream.write(String::from(reply).as_bytes())?;

    Ok(())
}
