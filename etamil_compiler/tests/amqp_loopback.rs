//! AMQP against a scripted peer on loopback.
//!
//! What this is for, and what it is not for.
//!
//! It drives the client through a whole conversation — handshake, declare,
//! publish with a confirm, get, ack — against a socket that plays the broker
//! from a script. That catches the things that are easy to get wrong and
//! invisible otherwise: a field in the wrong order, a bit packed from the wrong
//! end, a length written as a short where the specification says a long.
//!
//! What it cannot catch is a misreading of the specification, because the same
//! reading is on both sides of the socket. So the frames the client sends are
//! asserted **byte for byte against expectations written out by hand from the
//! specification's tables**, not against anything the client produced — and the
//! decisive check is still `tests/amqp.rs` against a real RabbitMQ, which CI
//! runs in a service container.
//!
//! No network, no broker, no feature flag: it runs in the ordinary suite.

#![cfg(not(target_family = "wasm"))]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use etamil_compiler::amqp::{Connection, Published};

const FRAME_END: u8 = 0xCE;

/// One frame, as the peer saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Frame {
    kind: u8,
    channel: u16,
    payload: Vec<u8>,
}

fn short_string(out: &mut Vec<u8>, text: &str) {
    out.push(text.len() as u8);
    out.extend_from_slice(text.as_bytes());
}

fn frame(kind: u8, channel: u16, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![kind];
    out.extend_from_slice(&channel.to_be_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out.push(FRAME_END);
    out
}

fn method(class: u16, which: u16) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&class.to_be_bytes());
    out.extend_from_slice(&which.to_be_bytes());
    out
}

fn read_frame(stream: &mut TcpStream) -> Option<Frame> {
    let mut head = [0u8; 7];
    stream.read_exact(&mut head).ok()?;
    let size = u32::from_be_bytes([head[3], head[4], head[5], head[6]]) as usize;
    let mut payload = vec![0u8; size];
    stream.read_exact(&mut payload).ok()?;
    let mut end = [0u8; 1];
    stream.read_exact(&mut end).ok()?;
    assert_eq!(
        end[0], FRAME_END,
        "the client did not end a frame with 0xCE"
    );
    Some(Frame {
        kind: head[0],
        channel: u16::from_be_bytes([head[1], head[2]]),
        payload,
    })
}

/// The peer. `replies` is what it sends after each frame it reads, in order;
/// every frame it read goes down `seen` for the test to inspect.
fn serve(listener: TcpListener, replies: Vec<Vec<u8>>, seen: Sender<Frame>) {
    let (mut stream, _) = listener.accept().expect("the client did not connect");

    let mut header = [0u8; 8];
    stream.read_exact(&mut header).expect("no protocol header");
    assert_eq!(
        &header, b"AMQP\x00\x00\x09\x01",
        "the protocol header names something other than AMQP 0-9-1"
    );

    // Connection.Start, which the broker sends unprompted once the header is in.
    let mut start = method(10, 10);
    start.push(0); // version-major
    start.push(9); // version-minor
    start.extend_from_slice(&0u32.to_be_bytes()); // server properties: empty table
    let mechanisms = b"PLAIN AMQPLAIN";
    start.extend_from_slice(&(mechanisms.len() as u32).to_be_bytes());
    start.extend_from_slice(mechanisms);
    let locales = b"en_US";
    start.extend_from_slice(&(locales.len() as u32).to_be_bytes());
    start.extend_from_slice(locales);
    stream.write_all(&frame(1, 0, &start)).unwrap();

    for reply in replies {
        let Some(got) = read_frame(&mut stream) else {
            return;
        };
        let _ = seen.send(got);
        if !reply.is_empty() {
            stream.write_all(&reply).unwrap();
        }
    }
    // Keep the socket open until the client is done with it.
    let _ = read_frame(&mut stream);
}

/// A broker that gets through the handshake, then replies as told.
fn broker(after_handshake: Vec<Vec<u8>>) -> (Connection, Receiver<Frame>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("no loopback port");
    let port = listener.local_addr().unwrap().port();
    let (sender, receiver) = channel();

    let mut tune = method(10, 30);
    tune.extend_from_slice(&0u16.to_be_bytes()); // channel-max
    tune.extend_from_slice(&131_072u32.to_be_bytes()); // frame-max
    tune.extend_from_slice(&0u16.to_be_bytes()); // heartbeat

    let mut open_ok = method(10, 41);
    short_string(&mut open_ok, "");

    let mut channel_ok = method(20, 11);
    channel_ok.extend_from_slice(&0u32.to_be_bytes()); // reserved longstr

    let handshake = vec![
        frame(1, 0, &tune),           // after Connection.StartOk
        Vec::new(),                   // Connection.TuneOk gets no reply
        frame(1, 0, &open_ok),        // after Connection.Open
        frame(1, 1, &channel_ok),     // after Channel.Open
        frame(1, 1, &method(85, 11)), // after Confirm.Select
    ];

    let mut replies = handshake;
    replies.extend(after_handshake);

    thread::spawn(move || serve(listener, replies, sender));

    let connection = Connection::open(&format!("amqp://app:s3cret@127.0.0.1:{port}/payments"))
        .expect("the handshake did not complete");
    (connection, receiver)
}

fn frames(receiver: &Receiver<Frame>, count: usize) -> Vec<Frame> {
    (0..count)
        .map(|n| {
            receiver
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap_or_else(|_| panic!("the client sent {n} frames, not {count}"))
        })
        .collect()
}

/// Every byte of the handshake, written out from the specification's tables
/// rather than from what the client produced.
#[test]
fn the_handshake_is_what_the_specification_says_it_is() {
    let (mut connection, seen) = broker(Vec::new());
    let sent = frames(&seen, 5);

    // Connection.StartOk: client-properties (table), mechanism (shortstr),
    // response (longstr), locale (shortstr). PLAIN's response is one string
    // with NUL separators, not two fields — the mistake that looks right.
    let mut start_ok = method(10, 11);
    start_ok.extend_from_slice(&0u32.to_be_bytes());
    short_string(&mut start_ok, "PLAIN");
    let secret = b"\x00app\x00s3cret";
    start_ok.extend_from_slice(&(secret.len() as u32).to_be_bytes());
    start_ok.extend_from_slice(secret);
    short_string(&mut start_ok, "en_US");
    assert_eq!(sent[0].payload, start_ok);
    assert_eq!(sent[0].channel, 0, "the handshake is on channel zero");

    // Connection.TuneOk echoes the broker's limits, and agrees to no heartbeat.
    let mut tune_ok = method(10, 31);
    tune_ok.extend_from_slice(&0u16.to_be_bytes());
    tune_ok.extend_from_slice(&131_072u32.to_be_bytes());
    tune_ok.extend_from_slice(&0u16.to_be_bytes());
    assert_eq!(sent[1].payload, tune_ok);

    // Connection.Open: the vhost out of the URL's path, then two reserved
    // fields. "payments", not "/payments" — the slash is the separator.
    let mut open = method(10, 40);
    short_string(&mut open, "payments");
    short_string(&mut open, "");
    open.push(0);
    assert_eq!(sent[2].payload, open);

    assert_eq!(sent[3].payload, method(20, 10));
    assert_eq!(sent[3].channel, 1, "the channel is opened as channel one");

    // Confirm.Select, with nowait off: the acknowledgement is the point.
    let mut confirm = method(85, 10);
    confirm.push(0);
    assert_eq!(sent[4].payload, confirm);

    let _ = connection.close();
}

#[test]
fn a_durable_queue_sets_the_bit_the_specification_puts_second() {
    let mut declare_ok = method(50, 11);
    short_string(&mut declare_ok, "payments");
    declare_ok.extend_from_slice(&7u32.to_be_bytes()); // message count
    declare_ok.extend_from_slice(&0u32.to_be_bytes()); // consumer count

    let (mut connection, seen) = broker(vec![frame(1, 1, &declare_ok)]);
    let waiting = connection.declare_queue("payments", true).unwrap();
    assert_eq!(waiting, 7, "the queue's depth comes back from DeclareOk");

    let sent = frames(&seen, 6);
    // reserved short, queue shortstr, bits, arguments table.
    // The bits, from the low end: passive, durable, exclusive, auto-delete,
    // no-wait. Durable is the second, so a durable queue is 0b10 — writing 1
    // there would declare a passive queue, which fails on one that is absent.
    let mut declare = method(50, 10);
    declare.extend_from_slice(&0u16.to_be_bytes());
    short_string(&mut declare, "payments");
    declare.push(0b10);
    declare.extend_from_slice(&0u32.to_be_bytes());
    assert_eq!(sent[5].payload, declare);

    let _ = connection.close();
}

#[test]
fn a_transient_queue_sets_no_bits() {
    let mut declare_ok = method(50, 11);
    short_string(&mut declare_ok, "scratch");
    declare_ok.extend_from_slice(&0u32.to_be_bytes());
    declare_ok.extend_from_slice(&0u32.to_be_bytes());

    let (mut connection, seen) = broker(vec![frame(1, 1, &declare_ok)]);
    connection.declare_queue("scratch", false).unwrap();
    let sent = frames(&seen, 6);

    let mut declare = method(50, 10);
    declare.extend_from_slice(&0u16.to_be_bytes());
    short_string(&mut declare, "scratch");
    declare.push(0b00);
    declare.extend_from_slice(&0u32.to_be_bytes());
    assert_eq!(sent[5].payload, declare);
    let _ = connection.close();
}

/// A publish is three frames — the method, a content header and a body — and
/// then it waits to be told the broker has it.
#[test]
fn a_publish_is_three_frames_and_then_a_wait() {
    let mut ack = method(60, 80);
    ack.extend_from_slice(&1u64.to_be_bytes()); // delivery tag
    ack.push(0); // not multiple

    let (mut connection, seen) = broker(vec![Vec::new(), Vec::new(), frame(1, 1, &ack)]);
    let answer = connection
        .publish(
            "",
            "payments",
            b"{\"amount\":1180}",
            "application/json",
            "c-1",
            "m-1",
        )
        .unwrap();
    assert_eq!(answer, Published::Confirmed);

    let sent = frames(&seen, 8);

    // Basic.Publish: reserved short, exchange, routing key, then the bits —
    // mandatory is the low bit, immediate the next. Mandatory is always set:
    // an exchange with nothing bound accepts and discards, and confirms alone
    // would answer yes.
    let mut publish = method(60, 40);
    publish.extend_from_slice(&0u16.to_be_bytes());
    short_string(&mut publish, "");
    short_string(&mut publish, "payments");
    publish.push(0b01);
    assert_eq!(sent[5].payload, publish);
    assert_eq!(sent[5].kind, 1);

    // The content header: class, weight, body size, property flags, and then
    // the properties present, in the order the flags are numbered — not the
    // order they were set.
    let mut header = method(60, 0); // class 60, weight 0
    header.extend_from_slice(&15u64.to_be_bytes()); // the body's length
    header.extend_from_slice(&((1u16 << 15) | (1 << 12) | (1 << 10) | (1 << 7)).to_be_bytes());
    short_string(&mut header, "application/json");
    header.push(2); // persistent, so a durable queue keeps it across a restart
    short_string(&mut header, "c-1");
    short_string(&mut header, "m-1");
    assert_eq!(sent[6].payload, header);
    assert_eq!(
        sent[6].kind, 2,
        "the second frame of a publish is the header"
    );

    assert_eq!(sent[7].kind, 3, "and the third is the body");
    assert_eq!(sent[7].payload, b"{\"amount\":1180}");

    let _ = connection.close();
}

/// Mandatory's whole point. The broker returns the message and *then* acks it,
/// so a client that only watched for the ack would call this a success.
#[test]
fn a_message_that_routed_nowhere_is_a_failure_despite_the_ack() {
    let mut returned = method(60, 50);
    returned.extend_from_slice(&312u16.to_be_bytes());
    short_string(&mut returned, "NO_ROUTE");
    short_string(&mut returned, "payments");
    short_string(&mut returned, "nobody.listening");

    let mut header = method(60, 0);
    header.extend_from_slice(&2u64.to_be_bytes());
    header.extend_from_slice(&0u16.to_be_bytes());

    let mut ack = method(60, 80);
    ack.extend_from_slice(&1u64.to_be_bytes());
    ack.push(0);

    let mut after_publish = frame(1, 1, &returned);
    after_publish.extend_from_slice(&frame(2, 1, &header));
    after_publish.extend_from_slice(&frame(3, 1, b"hi"));
    after_publish.extend_from_slice(&frame(1, 1, &ack));

    let (mut connection, _seen) = broker(vec![Vec::new(), Vec::new(), after_publish]);
    let answer = connection
        .publish("payments", "nobody.listening", b"hi", "", "", "")
        .unwrap();
    assert_eq!(
        answer,
        Published::Returned {
            code: 312,
            text: "NO_ROUTE".into()
        }
    );
    let _ = connection.close();
}

#[test]
fn a_broker_that_refuses_responsibility_says_so() {
    let mut nack = method(60, 120);
    nack.extend_from_slice(&1u64.to_be_bytes());
    nack.push(0);

    let (mut connection, _seen) = broker(vec![Vec::new(), Vec::new(), frame(1, 1, &nack)]);
    assert_eq!(
        connection.publish("", "q", b"x", "", "", "").unwrap(),
        Published::Nacked
    );
    let _ = connection.close();
}

/// Reading one back: the method frame says where it came from, the header says
/// how long it is, and the body follows in as many frames as it takes.
#[test]
fn a_message_comes_back_whole() {
    let mut get_ok = method(60, 71);
    get_ok.extend_from_slice(&9u64.to_be_bytes()); // delivery tag
    get_ok.push(1); // redelivered
    short_string(&mut get_ok, "orders");
    short_string(&mut get_ok, "paid");
    get_ok.extend_from_slice(&3u32.to_be_bytes()); // still waiting

    let body = b"{\"irn\":\"a1b2c3\"}";
    let mut header = method(60, 0);
    header.extend_from_slice(&(body.len() as u64).to_be_bytes());
    header.extend_from_slice(&((1u16 << 15) | (1 << 10)).to_be_bytes());
    short_string(&mut header, "application/json");
    short_string(&mut header, "corr-7");

    let mut reply = frame(1, 1, &get_ok);
    reply.extend_from_slice(&frame(2, 1, &header));
    // Split across two body frames, because a body larger than frame-max is
    // several and a client that assumed one would truncate it.
    reply.extend_from_slice(&frame(3, 1, &body[..5]));
    reply.extend_from_slice(&frame(3, 1, &body[5..]));

    let (mut connection, _seen) = broker(vec![reply]);
    let delivery = connection.get("orders").unwrap().expect("a message");

    assert_eq!(delivery.tag, 9);
    assert!(delivery.redelivered);
    assert_eq!(delivery.exchange, "orders");
    assert_eq!(delivery.routing_key, "paid");
    assert_eq!(delivery.waiting, 3);
    assert_eq!(delivery.content_type, "application/json");
    assert_eq!(delivery.correlation_id, "corr-7");
    assert_eq!(delivery.message_id, "");
    assert_eq!(delivery.body, body);

    let _ = connection.close();
}

/// An empty queue is not an error, and not an empty message either. இன்மை and
/// a message with no body are different answers to different questions.
#[test]
fn an_empty_queue_answers_nothing_rather_than_failing() {
    let mut empty = method(60, 72);
    short_string(&mut empty, "");
    let (mut connection, _seen) = broker(vec![frame(1, 1, &empty)]);
    assert_eq!(connection.get("orders").unwrap(), None);
    let _ = connection.close();
}

#[test]
fn acknowledging_names_the_delivery_and_not_the_queue() {
    let (mut connection, seen) = broker(vec![Vec::new()]);
    connection.ack(42).unwrap();

    let sent = frames(&seen, 6);
    let mut ack = method(60, 80);
    ack.extend_from_slice(&42u64.to_be_bytes());
    ack.push(0); // multiple off: this delivery, not every one up to it
    assert_eq!(sent[5].payload, ack);
    let _ = connection.close();
}

/// Requeue is the second bit, not the first — the first is `multiple`. Getting
/// those the wrong way round would requeue every unacknowledged delivery while
/// appearing to reject one.
#[test]
fn rejecting_puts_requeue_in_the_bit_the_specification_gives_it() {
    let (mut connection, seen) = broker(vec![Vec::new(), Vec::new()]);
    connection.nack(5, true).unwrap();
    connection.nack(6, false).unwrap();

    let sent = frames(&seen, 7);
    assert_eq!(sent[5].payload[4 + 8], 0b10, "requeue on, multiple off");
    assert_eq!(sent[6].payload[4 + 8], 0b00, "neither");
    let _ = connection.close();
}

/// What the broker said when it refused has to survive: a queue redeclared with
/// different durability says exactly that, and "unexpected method 20/40" would
/// throw away the only useful part.
#[test]
fn a_refusal_comes_back_carrying_what_the_broker_said() {
    let mut close = method(20, 40);
    close.extend_from_slice(&406u16.to_be_bytes());
    short_string(
        &mut close,
        "PRECONDITION_FAILED - inequivalent arg 'durable'",
    );
    close.extend_from_slice(&50u16.to_be_bytes()); // the class it objected to
    close.extend_from_slice(&10u16.to_be_bytes()); // and the method

    let (mut connection, _seen) = broker(vec![frame(1, 1, &close)]);
    let why = connection.declare_queue("payments", false).unwrap_err();
    assert!(why.contains("406"), "{why}");
    assert!(why.contains("inequivalent arg 'durable'"), "{why}");
}
