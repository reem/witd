extern crate http;
extern crate url;
extern crate curl;
extern crate serialize;
use std::io;
use std::str;
use std::io::File;
use curl::http;
use curl::http::Request;
use curl::ErrCode;
use serialize::json;
use serialize::json::Json;

enum WitRequestSpec {
    Message(String),
    Speech(Box<Reader+Send>, String)
}

struct WitRequest {
    sender: Sender<Result<Json,ErrCode>>,
    spec: WitRequestSpec
}

fn exec_request(request: Request) -> Result<Json,ErrCode> {
    request
        .header("Authorization", "Bearer 6FUG2L4PL5YFHDQYAOY6WCEA2EFDGFUV")
        .header("Accept", "application/vnd.wit.20140620+json")
    .exec().map(|resp| {
        let body = resp.get_body();
        let str = str::from_utf8(body.as_slice())
            .expect("Response was not valid UTF-8");
        json::from_str(str).unwrap()
    })
}

fn message_request(msg: String) -> Result<Json,ErrCode> {
    let mut init_req = http::handle();
    let req = init_req
        .get(format!("https://api.wit.ai/message?q={}", msg));
    exec_request(req)
}

fn speech_request(mut stream: Box<Reader>
                  , content_type: String) -> Result<Json,ErrCode> {
    let mut init_req = http::handle();
    let req = init_req.post("https://api.wit.ai/speech", &mut stream as &mut Reader)
        .content_type(content_type.as_slice())
        .chunked();
    exec_request(req)
}

fn result_fetcher(rx: &Receiver<WitRequest>) {
    loop {
        let WitRequest { sender: sender, spec: spec } = rx.recv();
        let result = match spec {
          Message(msg) => message_request(msg),
          Speech(stream, content_type) => speech_request(stream, content_type)
        };
        sender.send(result)
    }
}

fn main() {

    let (tx2, rx2): (Sender<WitRequest>, Receiver<WitRequest>) = channel();
    spawn(proc() {
        result_fetcher(&rx2);
    });

    for line in io::stdin().lines() {
        let path = Path::new("test.wav");
        match File::open(&path) {
            Ok(file) => {
                let (tx1, rx1): (Sender<Result<Json,ErrCode>>, Receiver<Result<Json,ErrCode>>) = channel();
                let spec = Speech(box file, "audio/wav".to_string());
                //let spec = Message(line.unwrap());
                let req = WitRequest{ sender: tx1, spec: spec };
                tx2.send(req);
                match rx1.recv() {
                    Ok(json) => {
                        println!("Result: {}", json);
                    }
                    Err(failure) => println!("Failure: {}", failure)
                }
            }
            Err(err) => println!("Error opening file {}", err)
        }
    }
}