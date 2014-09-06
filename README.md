# witd

`witd` is a light executable that makes it easy to use Wit.ai on a wide range of devices. It managesclient audio recording and communication wit Wit.ai.

## Why?

Wit.ai allows developers to build smart apps and devices. Developers have to do 3 things:

- record audio
- stream audio to Wit

## How to build

`witd` is written in [Rust][rust]. Build it with:

```bash
$ cargo build
```

## Start the server

Start the server:

```bash
$ HOST=0.0.0.0 PORT=9877 ./target/witd
```

## Send requests

### Speech request

For the moment, witd requires that you tell it when to start and stop recording. Moving forward, 2 new modes will be added:

- "silence detection" to stop the recording
- "hands-free" to start and stop the recording when user is speaking

Start recording audio and streaming it to wit.ai:

```bash
$ curl -X GET "http://localhost:9877/start&access_token=<YOUR_ACCESS_TOKEN>"
```

Stop recording audio and receive the wit.ai response:
```bash
$ curl -X GET "http://localhost:9877/stop"
{"_text":"Hello world","msg_id":"fbe2a1ff-3869-49d8-885d-67e23357ffdc","outcomes":[{"_text":"Hello world","confidence":0.263,"entities":{"location":[{"suggested":true,"value":"Hello world"}]},"intent":"get_weather"}]}
```

### Text request

witd can also directly send text requests to wit.ai:

```bash
$ curl -X GET "http://localhost:9877/text?q=Hello%20world&access_token=<YOUR_ACCESS_TOKEN>"
{"_text":"Hello world","msg_id":"fbe2a1ff-3869-49d8-885d-67e23357ffdc","outcomes":[{"_text":"Hello world","confidence":0.263,"entities":{"location":[{"suggested":true,"value":"Hello world"}]},"intent":"get_weather"}]}
```

## Running on Raspberry Pi

The easiest way to have witd running on a Raspberry Pi is to run the provided ARM binaries:

```bash
./witd-arm
```

### Building witd-arm for Raspberry Pi (for the brave)

The procedure below describes how to cross-compile witd-arm on a Debian host targeting Raspbian. It may work with other configurations but has not been tested yet.

1. Setup a Rust cross-compiler by following [these instructions](https://github.com/npryce/rusty-pi/blob/master/doc/compile-the-compiler.asciidoc).
2. Install the required libraries on the Raspberry Pi:
```bash
pi@raspberrypi ~$ sudo apt-get install libssl-dev libcurl4-openssl-dev libcrypto++-dev
```
3. Install sshfs so that the build script running on the host can access the precompiled libraries on the Raspberry Pi by mounting a remote filesystem:
```bash
sudo apt-get install sshfs
sudo modprobe fuse
sudo adduser `whoami` fuse
sudo chown root:fuse /dev/fuse
sudo chmod +x /bin/fusermount
newgrp fuse (or logout/login)
newgrp
```
4. Run the build script:
```bash
./raspbuild pi@192.168.1.54
```
where `pi` is a user on the Raspberry Pi and 192.168.1.54 is the IP of the Raspberry Pi. You need read access to /usr/lib/arm-linux-gnueabihf and /lib/arm-linux-gnueabihf on the Raspberry Pi (which is the case for the default user on Raspbian). You may be prompted for your Debian and/or Raspberry Pi password.

The resulting executable should be in `target/arm-unknown-linux-gnueabihf/witd`.

[rust]: http://rust-lang.org
