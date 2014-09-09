extern crate time;
extern crate curl;
extern crate http;
extern crate url;
extern crate serialize;
extern crate getopts;
use std::collections::HashMap;
use std::io;
use std::io::net::ip::{SocketAddr, IpAddr, Ipv4Addr};
use std::os;
use getopts::{optopt,optflag,getopts,OptGroup,usage,short_usage};

use http::server::{Config, Server, ResponseWriter};
use http::server::request::{AbsolutePath, Request, RequestUri};
use http::status::{BadRequest, MethodNotAllowed, InternalServerError};
use http::headers::content_type::MediaType;
use self::curl::ErrCode;
use serialize::json;
use serialize::json::Json;
mod wit;
mod mic;

#[deriving(Clone)]
struct HttpServer {
    host: IpAddr,
    port: u16,
    wit_tx: Sender<wit::WitCommand>
}

fn parse_query_params<'s>(uri: &'s str) -> HashMap<&'s str, &'s str> {
    let mut args = HashMap::<&'s str, &'s str>::new();
    let all_params: Vec<&str> = uri.split('&').collect();
    for param in all_params.iter() {
        let v_params:Vec<&str> = param.split('=').collect();
        let inserted = match v_params.as_slice() {
            [k] => args.insert(k, "true"),
            [k, v] => args.insert(k, v),
            [k, v, ..] => args.insert(k, v),
            _ => false
        };
        // println!("param {} inserted : {}", v_params, inserted);
    }
    return args;
}

impl Server for HttpServer {
    fn get_config(&self) -> Config {
        Config { bind_address: SocketAddr { ip: self.host, port: self.port } }
    }

    fn handle_request(&self, r: http::server::request::Request, w: &mut ResponseWriter) {
        w.headers.date = Some(time::now_utc());
        w.headers.content_type = Some(MediaType {
            type_: format!("application"),
            subtype: format!("json"),
            parameters: vec!((format!("charset"), format!("UTF-8")))
        });

        w.headers.server = Some(format!("witd 0.0.1"));


        println!("[http] request: {}", r.request_uri);
        match r.request_uri {
            AbsolutePath(ref uri) => {
                let uri_vec:Vec<&str> = uri.as_slice().split('?').collect();

                match uri_vec.as_slice() {
                    ["/text", ..args] => {
                        if args.len() == 0 {
                            w.write("params not found (token or q)".as_bytes())
                                .unwrap_or_else(|e| println!("could not write resp"));
                            return;
                        }

                        let params = parse_query_params(uri_vec[1]);
                        let token = params.find(&"access_token");
                        let text = params.find(&"q");

                        if token.is_none() || text.is_none() {
                            w.write("params not found (token or q)".as_bytes())
                                .unwrap_or_else(|e| println!("could not write resp"));
                            return;
                        }

                        let wit_rx = wit::interpret_string(&self.wit_tx,
                                                           token.unwrap().to_string(),
                                                           text.unwrap().to_string());
                        let json = wit_rx.recv();
                        println!("[http] recv from wit: {}", json);
                        if json.is_err() {
                            w.status = InternalServerError;
                            w.write(b"something went wrong, sowwy!");
                        } else {
                            w.write(format!("{}", json.unwrap()).as_bytes()).unwrap();
                        }
                    },
                    ["/start", ..args] => {
                        // async Wit start
                        if args.len() == 0 {
                            w.write("params not found (token)".as_bytes())
                                .unwrap_or_else(|e| println!("could not write resp"));
                            return;
                        }

                        let params = parse_query_params(uri_vec[1]);
                        let token = params.find(&"access_token");

                        if token.is_none() {
                            w.write("params not found (token)".as_bytes())
                                .unwrap_or_else(|e| println!("could not write resp"));
                            return;
                        }

                        let content_type =
                            format!("audio/raw;encoding=unsigned-integer;bits=16;rate=8000;endian=big");
                        wit::start_recording(&self.wit_tx,
                                             token.unwrap().to_string(),
                                             content_type);
                    },
                    ["/stop", ..args] => {
                        // sync Wit stop
                        let wit_rx = wit::stop_recording(&self.wit_tx);
                        let json = wit_rx.recv();
                        println!("[http] recv from wit: {}", json);
                        if json.is_err() {
                            w.status = InternalServerError;
                            w.write(b"something went wrong, sowwy!");
                        } else {
                            w.write(format!("{}", json.unwrap()).as_bytes()).unwrap();
                        }
                    },
                    _ => println!("unk uri: {}", uri)
                }
            }
            _ => println!("not absolute uri")
        };
    }
}

fn print_usage() {

}

fn main() {
    let args = os::args();
    let program = args[0].clone();

    let opts = [
        optflag("h", "help", "display this help message"),
        optflag("l", "list-input", "list input devices"),
        optopt("i", "input", "select input device", "1"),
        optopt("r", "rate", "set recording sample rate", "16000")
    ];

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!(f.to_string())
    };

    let host: IpAddr =
        from_str(os::getenv("WITD_HOST")
                 .unwrap_or("0.0.0.0".to_string())
                 .as_slice())
        .unwrap_or(Ipv4Addr(0,0,0,0));

    let port: u16 =
        from_str(os::getenv("WITD_PORT")
                 .unwrap_or("9877".to_string())
                 .as_slice())
        .unwrap_or(9877);

    // println!("{}, {}", matches.opt_present("l"), matches.opt_strs("input"));

    // before Wit is initialized
    if matches.opt_present("help") {
        println!("{}", usage("witd (https://wit.ai)", opts.as_slice()));
        return;
    }

    let input: Option<int> = matches.opt_str("input").and_then(|x| from_str(x.as_slice()));
    let rate: Option<f64> = matches.opt_str("rate").and_then(|x| from_str(x.as_slice()));

    let wit_tx = wit::init(wit::Options{input_device: input, sample_rate: rate});

    // after Wit is initialized
    if matches.opt_present("list-input") {
        wit::list_devices();
        return;
    }

    let server = HttpServer {
        host: host,
        port: port,
        wit_tx: wit_tx
    };

    println!("[witd] listening on {}:{}", host.to_string(), port);
    server.serve_forever();
}
