extern crate time;
extern crate http;
extern crate url;
extern crate getopts;
extern crate wit;
extern crate serialize;
use std::collections::HashMap;
use std::io::net::ip::{SocketAddr, IpAddr, Ipv4Addr};
use std::{os, io};
use getopts::{optopt,optflag,getopts,usage};
use serialize::json;
use std::io::MemWriter;


use http::server::{Config, Server, ResponseWriter};
use http::server::request::{AbsolutePath, Request};
use http::status::InternalServerError;
use http::headers::content_type::MediaType;

#[deriving(Clone)]
struct HttpServer {
    host: IpAddr,
    port: u16,
    wit_handle: wit::cmd::WitHandle,
    default_autoend: bool
}

fn parse_query_params<'s>(uri: &'s str) -> HashMap<&'s str, &'s str> {
    let mut args = HashMap::<&'s str, &'s str>::new();
    let all_params: Vec<&str> = uri.split('&').collect();
    for param in all_params.iter() {
        let v_params:Vec<&str> = param.split('=').collect();
        match v_params.as_slice() {
            [k] => args.insert(k, "true"),
            [k, v] => args.insert(k, v),
            [k, v, ..] => args.insert(k, v),
            _ => false
        };
    }
    return args;
}

fn opt_string_from_result(json_result: Result<json::Json, wit::cmd::RequestError>) -> Option<String> {
    json_result.ok().and_then(|json| {
        println!("[wit] received response: {}", json);
        let mut s = MemWriter::new();
        json.to_writer(&mut s as &mut io::Writer).unwrap();
        String::from_utf8(s.unwrap()).ok()
    })
}

fn write_resp(res: Result<json::Json, wit::cmd::RequestError>, w: &mut ResponseWriter) {
    match opt_string_from_result(res) {
        Some(s) => w.write(format!("{}", s).as_bytes()).unwrap(),
        None => {
            w.status = InternalServerError;
            w.write(b"something went wrong, sowwy!").unwrap();
        }
    }
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
                    ["/text", args..] => {
                        if args.len() == 0 {
                            w.write("params not found (token or q)".as_bytes())
                                .unwrap_or_else(|e| println!("could not write resp: {}", e));
                            return;
                        }

                        let params = parse_query_params(uri_vec[1]);
                        let token = params.find(&"access_token");
                        let text = params.find(&"q");

                        if token.is_none() || text.is_none() {
                            w.write("params not found (token or q)".as_bytes())
                                .unwrap_or_else(|e| println!("could not write resp: {}", e));
                            return;
                        }

                        let res = wit::cmd::text_query(
                            &self.wit_handle,
                            text.unwrap().to_string(),
                            token.unwrap().to_string()
                        );
                        write_resp(res, w);
                    },
                    ["/start", args..] => {
                        // async Wit start
                        if args.len() == 0 {
                            w.write("params not found (token)".as_bytes())
                                .unwrap_or_else(|e| println!("could not write resp: {}", e));
                            return;
                        }

                        let params = parse_query_params(uri_vec[1]);
                        let token = params.find(&"access_token");

                        if token.is_none() {
                            w.write("params not found (token)".as_bytes())
                                .unwrap_or_else(|e| println!("could not write resp: {}", e));
                            return;
                        }
                        let token = token.unwrap().to_string();

                        let autoend_enabled = params
                            .find(&"autoend")
                            .and_then(|x| {from_str(*x)})
                            .unwrap_or(self.default_autoend);

                        if autoend_enabled {
                            let res = wit::cmd::voice_query_auto(
                                &self.wit_handle,
                                token
                            );
                            write_resp(res, w);
                        } else {
                            wit::cmd::voice_query_start(
                                &self.wit_handle,
                                token
                            );
                        }
                    },
                    ["/stop", ..] => {
                        let res = wit::cmd::voice_query_stop(&self.wit_handle);
                        write_resp(res, w);
                    },
                    _ => println!("unk uri: {}", uri)
                }
            }
            _ => println!("not absolute uri")
        };
    }
}

fn main() {
    let args = os::args();

    let opts = [
        optflag("h", "help", "display this help message"),
        optopt("i", "input", "select input device", "default"),
        optopt("a", "host", "IP address to listen on", "0.0.0.0"),
        optopt("p", "port", "TCP port to listen on", "9877"),
        optopt("e", "autoend", "Enable end of speech detection", "false"),
        optopt("v", "verbosity", "Verbosity level", "3")
    ];

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!(f.to_string())
    };

    let host: IpAddr =
        from_str(os::getenv("WITD_HOST")
                 .or(matches.opt_str("host"))
                 .unwrap_or("0.0.0.0".to_string())
                 .as_slice())
        .unwrap_or(Ipv4Addr(0,0,0,0));

    let port: u16 =
        from_str(os::getenv("WITD_PORT")
                 .or(matches.opt_str("port"))
                 .unwrap_or("9877".to_string())
                 .as_slice())
        .unwrap_or(9877);

    let default_autoend: bool = matches
        .opt_str("autoend")
        .and_then(|x| {
            from_str(x.as_slice())
        })
        .unwrap_or(false);

    // println!("{}, {}", matches.opt_present("l"), matches.opt_strs("input"));

    // before Wit is initialized
    if matches.opt_present("help") {
        println!("{}", usage("witd (https://github.com/wit-ai/witd)", opts.as_slice()));
        return;
    }

    let device_opt = matches.opt_str("input");
    let verbosity = matches.opt_str("verbosity")
                        .and_then(|s| { from_str(s.as_slice()) })
                        .unwrap_or(3);
    let handle = wit::cmd::init(device_opt, verbosity);

    let server = HttpServer {
        host: host,
        port: port,
        wit_handle: handle,
        default_autoend: default_autoend
    };

    if verbosity > 0 {
        println!("[witd] listening on {}:{}", host.to_string(), port);
    }
    server.serve_forever();
}
