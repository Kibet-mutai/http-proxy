use rand::prelude::*;
use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
};

struct Connection {
    socket: UdpSocket,
}
#[derive(Debug)]
pub enum Error {
    IpResolveError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IpResolveError => {
                write!(f, "Error converting Ip address")
            }
        }
    }
}
pub struct Query;

impl Connection {
    pub fn bind() -> Self {
        let addr = Ipv4Addr::new(0, 0, 0, 0);
        let client_addr = SocketAddr::new(IpAddr::V4(addr), 3400);
        let socket = UdpSocket::bind(client_addr).expect("binding failed");
        Connection { socket }
    }

    pub fn connect_dns(&mut self, dns_addr: SocketAddr) {
        if let Err(e) = self.socket.connect(dns_addr) {
            println!("Connection error: {e}");
        }
    }
}

impl Query {
    pub fn new() -> Query {
        Query
    }
    fn build_query(hostname: &str) -> (Vec<u8>, u16) {
        let mut query: Vec<u8> = Vec::new();

        //have a random number for transaction Id
        //generate a random number using rnd
        let mut rng = rand::thread_rng();
        let y = rng.gen::<u16>();

        let transaction_id = y.to_be_bytes();

        query.extend_from_slice(&transaction_id);
        query.extend_from_slice(&[0x01, 0x00]);
        query.extend_from_slice(&[0x00, 0x01]);
        query.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let hostname = hostname.to_string().clone();

        for part in hostname.split(".") {
            query.push(part.len() as u8);
            query.extend_from_slice(part.as_bytes());
        }

        query.push(0);

        query.extend_from_slice(&[0x00, 0x01]);
        query.extend_from_slice(&[0x00, 0x01]);

        (query, y)
    }
    pub fn send_query(hostname: String) -> Result<Ipv4Addr, Error> {
        let mut socket = Connection::bind();
        let dns_ip = Ipv4Addr::new(8, 8, 8, 8);
        let dns_addr = SocketAddr::new(IpAddr::V4(dns_ip), 53);
        socket.connect_dns(dns_addr);
        let (query, tid) = Query::build_query(&hostname);
        socket.socket.send(&query).expect("Error sending message");
        let ip_offset = 12 + hostname.len() + 2 + 4;
        let mut buf = [0u8; 512];
        let (_, _) = socket.socket.recv_from(&mut buf).unwrap();
        let rtid = u16::from_be_bytes([buf[0], buf[1]]);
        if tid == rtid {
            //if type A record
            if buf[ip_offset + 2] == 0x00 && buf[ip_offset + 3] == 0x01 {
                let ip_addr = std::net::Ipv4Addr::new(
                    buf[ip_offset + 12],
                    buf[ip_offset + 13],
                    buf[ip_offset + 14],
                    buf[ip_offset + 15],
                );
                Ok(ip_addr)
            } else {
                Err(Error::IpResolveError)
            }
        } else {
            Err(Error::IpResolveError)
        }
    }
}
