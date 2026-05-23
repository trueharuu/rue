use engine_core::{board::Board, game::Game, piece::Piece};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

pub struct Server {
    pub addr: String,
    pub listener: TcpListener,
}

impl Server {
    #[must_use]
    pub async fn new(addr: String) -> Self {
        Self {
            addr: addr.clone(),
            listener: TcpListener::bind(&addr).await.unwrap(),
        }
    }

    pub async fn run(&self) {
        // println!("server running on {}", self.addr);
        loop {
            let (socket, _) = self.listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut conn = Conn::new(socket);
                conn.run().await;
            });
        }
    }
}

pub struct Conn {
    pub reader: BufReader<tokio::io::ReadHalf<TcpStream>>,
    pub writer: tokio::io::WriteHalf<TcpStream>,
    pub done: bool,
}

impl Conn {
    #[must_use]
    pub fn new(socket: TcpStream) -> Self {
        let (read_half, write_half) = tokio::io::split(socket);
        Self {
            reader: BufReader::new(read_half),
            writer: write_half,
            done: false,
        }
    }

    pub async fn run(&mut self) {
        // continuously accept and handle messages
        loop {
            if self.done {
                return;
            }
            // eprintln!("block A");
            let request = self.recv().await;
            // eprintln!("block B");
            let response = self.handle_one(request).await;
            // eprintln!("block C");
            self.send(response).await;
            // eprintln!("block D");
        }
    }

    // messages are `\n` suffixed json
    pub async fn recv(&mut self) -> Option<Request> {
        let mut line = String::new();
        self.reader.read_line(&mut line).await.unwrap();

        if line.is_empty() {
            return None;
        }
        eprintln!("recieving {line}");
        serde_json::from_str(&line)
            .inspect_err(|e| eprintln!("{e:?}"))
            .ok()
    }

    pub async fn send(&mut self, response: Response) {
        eprintln!("sending {response:?}");
        let mut buf = serde_json::to_vec(&response).unwrap();
        buf.push(b'\n');
        self.writer.write_all(&buf).await.ok();
        self.writer.flush().await.ok();
    }

    #[allow(clippy::unused_async)]
    pub async fn handle_one(&mut self, request: Option<Request>) -> Response {
        let Some(request) = request else {
            self.done = true;
            return Response::Exit;
        };
        eprintln!("RECV {request:?}");

        match request {
            Request::Ping => Response::Pong,
            Request::Setup => Response::Setup,
            Request::Path {
                board,
                hold,
                queue,
                combo,
                b2b,
                current,
                incoming_garbage,
            } => {
                let b = dec_board(board);
                let mut g = Game::new(b.clone(), current, queue.to_vec());
                g.hold = hold;
                g.b2b = b2b.max(0) as u8;
                g.combo = combo.max(0) as u32;
                g.pending_garbage = incoming_garbage;

                Response::Fail("dead".to_string())
            }
            Request::Exit => {
                self.done = true;
                self.writer.shutdown().await.unwrap();
                Response::Exit
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "packet", content = "data")]
pub enum Request {
    Ping,
    Setup,
    Exit,
    Path {
        board: String,
        combo: i8,
        b2b: i16,
        incoming_garbage: u8,
        hold: Option<Piece>,
        current: Piece,
        queue: Vec<Piece>,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "packet", content = "data")]
pub enum Response {
    Pong,
    Setup,
    Exit,
    Fail(String),
    Path {
        finesse: Vec<String>,
        piece: Piece,
        // spin: Spin,
    },
}

pub fn dec_board(s: String) -> Board {
    let mut board = Board::new();
    for (y, line) in s.split('|').enumerate() {
        for (x, c) in line.chars().enumerate() {
            if c == 'X' {
                board.set(x, y);
            }
        }
    }
    board
}
