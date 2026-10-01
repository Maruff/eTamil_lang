// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! AMQP 0-9-1, over the protocol it actually speaks.
//!
//! Implemented here rather than taken from a crate, for the reason `redis.rs`
//! gives: the subset a program needs is small, the TCP stack was already
//! present, and the crates that speak AMQP carry an async runtime to put a few
//! hundred bytes down a socket. The same trade, and the same answer.
//!
//! ## What "sent" is allowed to mean
//!
//! The one decision everything else here follows from. `Basic.Publish` has no
//! reply: writing it to the socket tells you the bytes left, and nothing about
//! whether the broker has them. For a queue carrying payment instructions that
//! is not a useful thing to be told, so the channel turns on **publisher
//! confirms** at open and every publish waits for the broker's `Basic.Ack`
//! before answering. A publish that returns `சரி` means the broker has taken
//! responsibility for the message.
//!
//! Publishes are also **mandatory**, which is the other half of the same
//! question. Confirms say the broker accepted the message; they do not say it
//! reached a queue. An exchange with nothing bound to the routing key accepts
//! and discards, and then acks — so confirms alone answer "yes" for a message
//! that went nowhere. With `mandatory` the broker sends `Basic.Return` first,
//! and this reports that as a failure rather than a success with a footnote.
//!
//! ## One connection, one channel
//!
//! The same shape `ரெடிஸ்_இணை` has, for the same reason: a channel carries
//! per-channel state — confirm mode, unacknowledged deliveries, prefetch — so
//! handing one to two requests has the hazard of a shared transaction. Until
//! leases exist, `செய்தி_இணை` opens its own and holds it for the life of the
//! program.
//!
//! ## What is not here
//!
//! No `amqps`. TLS belongs in this eventually and is not half-written here;
//! until then a broker reached over an untrusted network wants a tunnel, and
//! `docs/backend/AMQP.md` says so rather than leaving it to be discovered.
//!
//! No `Basic.Consume` — the push delivery mode, where the broker streams
//! messages at an open socket. `Basic.Get` pulls one at a time, which is what a
//! program with a request-shaped life can actually use; a consumer loop wants a
//! worker that outlives a request, and that is a design, not an omission to
//! paper over.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const FRAME_METHOD: u8 = 1;
const FRAME_HEADER: u8 = 2;
const FRAME_BODY: u8 = 3;
const FRAME_HEARTBEAT: u8 = 8;
const FRAME_END: u8 = 0xCE;

/// Everything runs on channel 1. See the note above about why there is one.
const CHANNEL: u16 = 1;

const CONNECTION: u16 = 10;
const CHANNEL_CLASS: u16 = 20;
const EXCHANGE: u16 = 40;
const QUEUE: u16 = 50;
const BASIC: u16 = 60;
const CONFIRM: u16 = 85;

/// A message that came off a queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    pub tag: u64,
    pub redelivered: bool,
    pub exchange: String,
    pub routing_key: String,
    pub waiting: u32,
    pub content_type: String,
    pub correlation_id: String,
    pub message_id: String,
    pub body: Vec<u8>,
}

/// What a publish is told, which is more than "the bytes left".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Published {
    /// The broker has it and has said so.
    Confirmed,
    /// The broker refused responsibility for it.
    Nacked,
    /// Accepted by the exchange and routed nowhere, which `mandatory` reports.
    Returned { code: u16, text: String },
}

/// Where to connect, taken apart from an `amqp://` URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub vhost: String,
}

impl Address {
    /// `amqp://user:pass@host:port/vhost`, with every part optional.
    ///
    /// An empty path is the default vhost `/`, not an empty vhost — those are
    /// different vhosts and a broker will say so. `%2f` in the path is the one
    /// escape that matters, because the usual vhost name is a character that
    /// cannot appear in a path segment.
    pub fn parse(url: &str) -> Result<Self, String> {
        let rest = url.strip_prefix("amqp://").ok_or_else(|| {
            if url.starts_with("amqps://") {
                "amqps இன்னும் இல்லை  (TLS is not implemented here yet; see docs/backend/AMQP.md)"
                    .to_string()
            } else {
                format!(
                    "'{}' ஒரு amqp:// முகவரி அல்ல  ('{}' is not an amqp:// URL)",
                    url, url
                )
            }
        })?;

        let (authority, path) = match rest.find('/') {
            Some(at) => (&rest[..at], &rest[at + 1..]),
            None => (rest, ""),
        };

        let (credentials, host_port) = match authority.rfind('@') {
            Some(at) => (&authority[..at], &authority[at + 1..]),
            None => ("", authority),
        };

        let (user, password) = match credentials.find(':') {
            Some(at) => (&credentials[..at], &credentials[at + 1..]),
            None if credentials.is_empty() => ("guest", "guest"),
            None => (credentials, ""),
        };

        let (host, port) = match host_port.rfind(':') {
            Some(at) => {
                let port = host_port[at + 1..].parse::<u16>().map_err(|_| {
                    format!(
                        "'{}' ஒரு துறை எண் அல்ல  ('{}' is not a port number)",
                        &host_port[at + 1..],
                        &host_port[at + 1..]
                    )
                })?;
                (&host_port[..at], port)
            }
            None => (host_port, 5672),
        };

        let vhost = if path.is_empty() {
            "/".to_string()
        } else {
            path.replace("%2F", "/").replace("%2f", "/")
        };

        Ok(Address {
            host: if host.is_empty() {
                "localhost".into()
            } else {
                host.to_string()
            },
            port,
            user: user.to_string(),
            password: password.to_string(),
            vhost,
        })
    }
}

// --- writing ----------------------------------------------------------------

fn short_string(out: &mut Vec<u8>, text: &str) {
    let bytes = text.as_bytes();
    let length = bytes.len().min(255);
    out.push(length as u8);
    out.extend_from_slice(&bytes[..length]);
}

fn long_string(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}

/// An empty field table, which is four zero bytes: the table's own length.
fn empty_table(out: &mut Vec<u8>) {
    out.extend_from_slice(&0u32.to_be_bytes());
}

fn method(class: u16, method: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(&class.to_be_bytes());
    out.extend_from_slice(&method.to_be_bytes());
    out
}

// --- reading ----------------------------------------------------------------

struct Frame {
    kind: u8,
    payload: Vec<u8>,
}

/// A cursor over a method's arguments.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Reader { bytes, at: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(count).ok_or_else(short)?;
        let slice = self.bytes.get(self.at..end).ok_or_else(short)?;
        self.at = end;
        Ok(slice)
    }

    fn octet(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn short(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn long(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn long_long(&mut self) -> Result<u64, String> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn short_string(&mut self) -> Result<String, String> {
        let length = self.octet()? as usize;
        Ok(String::from_utf8_lossy(self.take(length)?).into_owned())
    }

    /// Skip a field table without decoding it. Its first four bytes are its own
    /// length, which is the whole reason this can be skipped rather than
    /// implemented: nothing here reads a server property.
    fn skip_table(&mut self) -> Result<(), String> {
        let length = self.long()? as usize;
        let _ = self.take(length)?;
        Ok(())
    }
}

fn short() -> String {
    "AMQP சட்டகம் முழுமையற்றது  (an AMQP frame ended in the middle of a value)".to_string()
}

// --- the connection ---------------------------------------------------------

pub struct Connection {
    stream: TcpStream,
    url: String,
    /// The broker's limit on a single body frame, less the 8 bytes of framing.
    body_max: usize,
    /// Publisher confirms are numbered from 1 by the broker, in publish order.
    published: u64,
}

/// Written by hand rather than derived, as `redis.rs` does and for the same
/// reason: a socket has no useful Debug, and the one thing worth printing
/// about a connection is where it goes. The password is not in it.
impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match Address::parse(&self.url) {
            Ok(address) => write!(
                f,
                "AMQP({}:{}{})",
                address.host, address.port, address.vhost
            ),
            Err(_) => write!(f, "AMQP(?)"),
        }
    }
}

impl Connection {
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Connect, authenticate, open a channel and turn on publisher confirms.
    pub fn open(url: &str) -> Result<Self, String> {
        let address = Address::parse(url)?;
        let stream = TcpStream::connect((address.host.as_str(), address.port)).map_err(|e| {
            format!(
                "தரகர் '{}:{}' இணைக்க முடியவில்லை  (cannot reach the broker at '{}:{}'): {}",
                address.host, address.port, address.host, address.port, e
            )
        })?;

        // A broker that accepts the connection and then says nothing would
        // otherwise hold a worker for as long as it liked.
        let timeout = Some(Duration::from_secs(30));
        let _ = stream.set_read_timeout(timeout);
        let _ = stream.set_write_timeout(timeout);

        let mut connection = Connection {
            stream,
            url: url.to_string(),
            body_max: 4096,
            published: 0,
        };
        connection.handshake(&address)?;
        Ok(connection)
    }

    fn handshake(&mut self, address: &Address) -> Result<(), String> {
        // The protocol header is the one thing that is not a frame: eight bytes
        // naming the protocol and the version, before anything else is said.
        self.stream
            .write_all(b"AMQP\x00\x00\x09\x01")
            .map_err(|e| transport("the protocol header", e))?;

        let (class, _method, body) = self.expect_method(&[(CONNECTION, 10)])?;
        debug_assert_eq!(class, CONNECTION);
        let mut reader = Reader::new(&body);
        let _version_major = reader.octet()?;
        let _version_minor = reader.octet()?;
        reader.skip_table()?; // server properties
        let mechanisms = {
            let length = reader.long()? as usize;
            String::from_utf8_lossy(reader.take(length)?).into_owned()
        };
        if !mechanisms.split(' ').any(|one| one == "PLAIN") {
            return Err(format!(
                "தரகர் PLAIN ஐ ஏற்கவில்லை; அது சொல்வது: {}  \
                 (the broker does not offer PLAIN, which is the only mechanism here)",
                mechanisms
            ));
        }

        // PLAIN is a single string with NUL separators, not two fields.
        let mut secret = Vec::with_capacity(address.user.len() + address.password.len() + 2);
        secret.push(0);
        secret.extend_from_slice(address.user.as_bytes());
        secret.push(0);
        secret.extend_from_slice(address.password.as_bytes());

        let mut start_ok = method(CONNECTION, 11);
        empty_table(&mut start_ok); // client properties
        short_string(&mut start_ok, "PLAIN");
        long_string(&mut start_ok, &secret);
        short_string(&mut start_ok, "en_US");
        self.send(FRAME_METHOD, 0, &start_ok)?;

        let (_, _, tune) = self.expect_method(&[(CONNECTION, 30)])?;
        let mut reader = Reader::new(&tune);
        let channel_max = reader.short()?;
        let frame_max = reader.long()?;
        let _heartbeat = reader.short()?;

        // 8 bytes of framing: the 7-byte header and the frame-end octet.
        self.body_max = (frame_max as usize).saturating_sub(8).max(1024);

        let mut tune_ok = method(CONNECTION, 31);
        tune_ok.extend_from_slice(&channel_max.to_be_bytes());
        tune_ok.extend_from_slice(&frame_max.to_be_bytes());
        // Heartbeats off. A heartbeat is a timer, and nothing in the VM runs
        // while an eTamil program is between calls — so agreeing to one would
        // be agreeing to something this cannot do, and the broker would drop
        // the connection for missing it.
        tune_ok.extend_from_slice(&0u16.to_be_bytes());
        self.send(FRAME_METHOD, 0, &tune_ok)?;

        let mut open = method(CONNECTION, 40);
        short_string(&mut open, &address.vhost);
        short_string(&mut open, ""); // reserved
        open.push(0); // reserved
        self.send(FRAME_METHOD, 0, &open)?;
        let _ = self.expect_method(&[(CONNECTION, 41)])?;

        self.send(FRAME_METHOD, CHANNEL, &method(CHANNEL_CLASS, 10))?;
        let _ = self.expect_method(&[(CHANNEL_CLASS, 11)])?;

        // Confirms, at open rather than on demand: a publish that cannot be
        // confirmed is a publish whose answer would have to be a guess.
        let mut confirm = method(CONFIRM, 10);
        confirm.push(0); // nowait = false
        self.send(FRAME_METHOD, CHANNEL, &confirm)?;
        let _ = self.expect_method(&[(CONFIRM, 11)])?;

        Ok(())
    }

    /// Declare a queue, creating it if it is not there.
    ///
    /// A durable queue survives a broker restart; its messages do so only if
    /// they were published persistently, which `publish` always does.
    pub fn declare_queue(&mut self, name: &str, durable: bool) -> Result<u32, String> {
        let mut declare = method(QUEUE, 10);
        declare.extend_from_slice(&0u16.to_be_bytes()); // reserved
        short_string(&mut declare, name);
        // passive, durable, exclusive, auto-delete, no-wait — packed into bits
        // in that order from the low end.
        declare.push(u8::from(durable) << 1);
        empty_table(&mut declare);
        self.send(FRAME_METHOD, CHANNEL, &declare)?;

        let (_, _, body) = self.expect_method(&[(QUEUE, 11)])?;
        let mut reader = Reader::new(&body);
        let _name = reader.short_string()?;
        reader.long() // message count
    }

    /// Declare an exchange. `kind` is `direct`, `topic`, `fanout` or `headers`.
    pub fn declare_exchange(
        &mut self,
        name: &str,
        kind: &str,
        durable: bool,
    ) -> Result<(), String> {
        let mut declare = method(EXCHANGE, 10);
        declare.extend_from_slice(&0u16.to_be_bytes()); // reserved
        short_string(&mut declare, name);
        short_string(&mut declare, kind);
        declare.push(u8::from(durable) << 1);
        empty_table(&mut declare);
        self.send(FRAME_METHOD, CHANNEL, &declare)?;
        self.expect_method(&[(EXCHANGE, 11)]).map(|_| ())
    }

    pub fn bind(&mut self, queue: &str, exchange: &str, routing_key: &str) -> Result<(), String> {
        let mut bind = method(QUEUE, 20);
        bind.extend_from_slice(&0u16.to_be_bytes()); // reserved
        short_string(&mut bind, queue);
        short_string(&mut bind, exchange);
        short_string(&mut bind, routing_key);
        bind.push(0); // no-wait
        empty_table(&mut bind);
        self.send(FRAME_METHOD, CHANNEL, &bind)?;
        self.expect_method(&[(QUEUE, 21)]).map(|_| ())
    }

    /// Publish, and wait to be told the broker has it.
    ///
    /// Persistent and mandatory, neither of which is optional here. Persistent
    /// because a durable queue whose messages are transient loses them on a
    /// restart, which is the failure that looks like durability right up until
    /// it matters. Mandatory because an exchange with nothing bound accepts and
    /// discards, and confirms would say yes.
    pub fn publish(
        &mut self,
        exchange: &str,
        routing_key: &str,
        body: &[u8],
        content_type: &str,
        correlation_id: &str,
        message_id: &str,
    ) -> Result<Published, String> {
        let mut publish = method(BASIC, 40);
        publish.extend_from_slice(&0u16.to_be_bytes()); // reserved
        short_string(&mut publish, exchange);
        short_string(&mut publish, routing_key);
        publish.push(0b01); // mandatory, not immediate
        self.send(FRAME_METHOD, CHANNEL, &publish)?;

        let mut header = Vec::with_capacity(32);
        header.extend_from_slice(&BASIC.to_be_bytes());
        header.extend_from_slice(&0u16.to_be_bytes()); // weight
        header.extend_from_slice(&(body.len() as u64).to_be_bytes());

        // Property flags, from bit 15 down: content-type, delivery-mode,
        // correlation-id, message-id. The properties follow in the same order,
        // and the order is the specification's, not a choice.
        let mut flags: u16 = 0;
        if !content_type.is_empty() {
            flags |= 1 << 15;
        }
        flags |= 1 << 12; // delivery-mode, always written
        if !correlation_id.is_empty() {
            flags |= 1 << 10;
        }
        if !message_id.is_empty() {
            flags |= 1 << 7;
        }
        header.extend_from_slice(&flags.to_be_bytes());
        if !content_type.is_empty() {
            short_string(&mut header, content_type);
        }
        header.push(2); // persistent
        if !correlation_id.is_empty() {
            short_string(&mut header, correlation_id);
        }
        if !message_id.is_empty() {
            short_string(&mut header, message_id);
        }
        self.send(FRAME_HEADER, CHANNEL, &header)?;

        for chunk in body.chunks(self.body_max).take(usize::MAX) {
            self.send(FRAME_BODY, CHANNEL, chunk)?;
        }
        // A zero-length body is still a message, and it has no body frames.
        if body.is_empty() {
            // nothing to send; the header's body-size of 0 says so
        }

        self.published += 1;
        self.await_confirmation(self.published)
    }

    /// Read until this publish is answered for.
    ///
    /// `Basic.Return` arrives before the ack when a mandatory message routed
    /// nowhere, so the return is remembered and the ack that follows does not
    /// overwrite it. A `multiple` ack covers everything up to its tag, which is
    /// why the comparison is `>=` rather than `==`.
    fn await_confirmation(&mut self, tag: u64) -> Result<Published, String> {
        let mut returned: Option<Published> = None;
        loop {
            let frame = self.read_frame()?;
            match frame.kind {
                FRAME_HEARTBEAT => continue,
                FRAME_HEADER | FRAME_BODY => continue, // the returned message's own
                FRAME_METHOD => {}
                other => {
                    return Err(format!(
                        "எதிர்பாராத சட்டக வகை {}  (unexpected frame type {})",
                        other, other
                    ));
                }
            }

            let mut reader = Reader::new(&frame.payload);
            let class = reader.short()?;
            let which = reader.short()?;
            let rest = &frame.payload[4..];

            match (class, which) {
                (BASIC, 50) => {
                    let mut body = Reader::new(rest);
                    let code = body.short()?;
                    let text = body.short_string()?;
                    returned = Some(Published::Returned { code, text });
                }
                (BASIC, 80) => {
                    let mut body = Reader::new(rest);
                    if body.long_long()? >= tag {
                        return Ok(returned.unwrap_or(Published::Confirmed));
                    }
                }
                (BASIC, 120) => {
                    let mut body = Reader::new(rest);
                    if body.long_long()? >= tag {
                        return Ok(returned.unwrap_or(Published::Nacked));
                    }
                }
                (CHANNEL_CLASS, 40) | (CONNECTION, 50) => return Err(closed(rest)?),
                _ => continue,
            }
        }
    }

    /// Take one message off a queue, or nothing if it is empty.
    ///
    /// Not auto-acknowledged: the message stays the broker's responsibility
    /// until `ack`, so a program that reads one and then fails leaves it to be
    /// delivered again. Auto-acknowledging would make `செய்தி_பெறு` a function
    /// that can lose a payment instruction by being interrupted.
    pub fn get(&mut self, queue: &str) -> Result<Option<Delivery>, String> {
        let mut get = method(BASIC, 70);
        get.extend_from_slice(&0u16.to_be_bytes()); // reserved
        short_string(&mut get, queue);
        get.push(0); // no-ack = false
        self.send(FRAME_METHOD, CHANNEL, &get)?;

        let (_, which, body) = self.expect_method(&[(BASIC, 71), (BASIC, 72)])?;
        if which == 72 {
            return Ok(None);
        }

        let mut reader = Reader::new(&body);
        let tag = reader.long_long()?;
        let redelivered = reader.octet()? & 1 == 1;
        let exchange = reader.short_string()?;
        let routing_key = reader.short_string()?;
        let waiting = reader.long()?;

        let header = self.read_frame()?;
        if header.kind != FRAME_HEADER {
            return Err("உடல் தலைப்பு வரவில்லை  (the broker sent no content header)".to_string());
        }
        let (size, content_type, correlation_id, message_id) = read_properties(&header.payload)?;

        let mut content = Vec::with_capacity(size as usize);
        while (content.len() as u64) < size {
            let frame = self.read_frame()?;
            match frame.kind {
                FRAME_BODY => content.extend_from_slice(&frame.payload),
                FRAME_HEARTBEAT => continue,
                _ => return Err("உடல் சட்டகம் எதிர்பார்க்கப்பட்டது  (expected a body frame)".to_string()),
            }
        }

        Ok(Some(Delivery {
            tag,
            redelivered,
            exchange,
            routing_key,
            waiting,
            content_type,
            correlation_id,
            message_id,
            body: content,
        }))
    }

    /// Done with it: the broker may forget it.
    pub fn ack(&mut self, tag: u64) -> Result<(), String> {
        let mut ack = method(BASIC, 80);
        ack.extend_from_slice(&tag.to_be_bytes());
        ack.push(0); // not multiple
        self.send(FRAME_METHOD, CHANNEL, &ack)
    }

    /// Not done with it. `requeue` puts it back for another attempt; without it
    /// the message is dead-lettered if the queue says where, and dropped if it
    /// does not — so a queue with no dead-letter exchange discards on reject,
    /// which is worth knowing before rejecting anything.
    pub fn nack(&mut self, tag: u64, requeue: bool) -> Result<(), String> {
        let mut nack = method(BASIC, 120);
        nack.extend_from_slice(&tag.to_be_bytes());
        nack.push(u8::from(requeue) << 1); // multiple = 0, requeue
        self.send(FRAME_METHOD, CHANNEL, &nack)
    }

    /// Close the channel and the connection, politely, so the broker does not
    /// have to notice the socket going away.
    pub fn close(&mut self) -> Result<(), String> {
        let mut close = method(CHANNEL_CLASS, 40);
        close.extend_from_slice(&200u16.to_be_bytes());
        short_string(&mut close, "");
        close.extend_from_slice(&0u16.to_be_bytes());
        close.extend_from_slice(&0u16.to_be_bytes());
        self.send(FRAME_METHOD, CHANNEL, &close)?;
        let _ = self.expect_method(&[(CHANNEL_CLASS, 41)]);

        let mut bye = method(CONNECTION, 50);
        bye.extend_from_slice(&200u16.to_be_bytes());
        short_string(&mut bye, "");
        bye.extend_from_slice(&0u16.to_be_bytes());
        bye.extend_from_slice(&0u16.to_be_bytes());
        self.send(FRAME_METHOD, 0, &bye)?;
        let _ = self.expect_method(&[(CONNECTION, 51)]);
        Ok(())
    }

    // --- frames ---

    fn send(&mut self, kind: u8, channel: u16, payload: &[u8]) -> Result<(), String> {
        let mut frame = Vec::with_capacity(payload.len() + 8);
        frame.push(kind);
        frame.extend_from_slice(&channel.to_be_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(payload);
        frame.push(FRAME_END);
        self.stream
            .write_all(&frame)
            .map_err(|e| transport("a frame", e))
    }

    fn read_frame(&mut self) -> Result<Frame, String> {
        let mut head = [0u8; 7];
        self.stream
            .read_exact(&mut head)
            .map_err(|e| transport("a frame header", e))?;
        let kind = head[0];
        let size = u32::from_be_bytes([head[3], head[4], head[5], head[6]]) as usize;

        let mut payload = vec![0u8; size];
        self.stream
            .read_exact(&mut payload)
            .map_err(|e| transport("a frame body", e))?;

        let mut end = [0u8; 1];
        self.stream
            .read_exact(&mut end)
            .map_err(|e| transport("a frame end", e))?;
        if end[0] != FRAME_END {
            return Err(format!(
                "சட்டகம் {:#04x} இல் முடிந்தது, {:#04x} அல்ல  \
                 (a frame ended with {:#04x} rather than {:#04x}, so the stream is out of step)",
                end[0], FRAME_END, end[0], FRAME_END
            ));
        }
        Ok(Frame { kind, payload })
    }

    /// Read until one of `wanted` arrives, stepping over heartbeats.
    ///
    /// A `Channel.Close` or `Connection.Close` instead is the broker refusing,
    /// and it carries a reason — a queue redeclared with different durability
    /// says exactly that. Reporting "unexpected method 20/40" instead of what
    /// the broker said would throw away the only useful part.
    fn expect_method(&mut self, wanted: &[(u16, u16)]) -> Result<(u16, u16, Vec<u8>), String> {
        loop {
            let frame = self.read_frame()?;
            if frame.kind == FRAME_HEARTBEAT {
                continue;
            }
            if frame.kind != FRAME_METHOD {
                continue;
            }
            let mut reader = Reader::new(&frame.payload);
            let class = reader.short()?;
            let which = reader.short()?;
            let rest = frame.payload[4..].to_vec();

            if wanted.contains(&(class, which)) {
                return Ok((class, which, rest));
            }
            if (class, which) == (CHANNEL_CLASS, 40) || (class, which) == (CONNECTION, 50) {
                return Err(closed(&rest)?);
            }
        }
    }
}

/// What the broker said when it closed something.
fn closed(body: &[u8]) -> Result<String, String> {
    let mut reader = Reader::new(body);
    let code = reader.short()?;
    let text = reader.short_string()?;
    Ok(format!(
        "தரகர் மறுத்தது {}: {}  (the broker refused: {} {})",
        code, text, code, text
    ))
}

fn transport(what: &str, error: std::io::Error) -> String {
    format!(
        "AMQP {} அனுப்ப/பெற முடியவில்லை  (cannot read or write {}): {}",
        what, what, error
    )
}

/// Body size and the three properties worth carrying back, out of a content
/// header. The rest are stepped over in the order the specification fixes.
fn read_properties(payload: &[u8]) -> Result<(u64, String, String, String), String> {
    let mut reader = Reader::new(payload);
    let _class = reader.short()?;
    let _weight = reader.short()?;
    let size = reader.long_long()?;
    let flags = reader.short()?;

    let mut content_type = String::new();
    let mut correlation_id = String::new();
    let mut message_id = String::new();

    if flags & (1 << 15) != 0 {
        content_type = reader.short_string()?;
    }
    if flags & (1 << 14) != 0 {
        let _encoding = reader.short_string()?;
    }
    if flags & (1 << 13) != 0 {
        reader.skip_table()?;
    }
    if flags & (1 << 12) != 0 {
        let _delivery_mode = reader.octet()?;
    }
    if flags & (1 << 11) != 0 {
        let _priority = reader.octet()?;
    }
    if flags & (1 << 10) != 0 {
        correlation_id = reader.short_string()?;
    }
    if flags & (1 << 9) != 0 {
        let _reply_to = reader.short_string()?;
    }
    if flags & (1 << 8) != 0 {
        let _expiration = reader.short_string()?;
    }
    if flags & (1 << 7) != 0 {
        message_id = reader.short_string()?;
    }

    Ok((size, content_type, correlation_id, message_id))
}

/// The properties a publish may carry, read off an eTamil record.
pub fn properties(record: &HashMap<String, crate::vm::Value>) -> (String, String, String) {
    let pick = |tamil: &str, english: &str| -> String {
        record
            .get(tamil)
            .or_else(|| record.get(english))
            .map(ToString::to_string)
            .unwrap_or_default()
    };
    (
        pick("வகை", "contentType"),
        pick("தொடர்பு", "correlationId"),
        pick("அடையாளம்", "messageId"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_comes_apart_the_way_the_defaults_say() {
        let plain = Address::parse("amqp://localhost").unwrap();
        assert_eq!(plain.port, 5672);
        assert_eq!(plain.user, "guest");
        assert_eq!(plain.password, "guest");
        // An empty path is the default vhost, which is named "/", not "".
        assert_eq!(plain.vhost, "/");

        let full = Address::parse("amqp://app:s3cret@broker.example:5673/payments").unwrap();
        assert_eq!(
            full,
            Address {
                host: "broker.example".into(),
                port: 5673,
                user: "app".into(),
                password: "s3cret".into(),
                vhost: "payments".into(),
            }
        );
    }

    /// The usual vhost name is a character that cannot appear in a path
    /// segment, so this escape is not an edge case — it is how most brokers
    /// are addressed.
    #[test]
    fn the_escaped_default_vhost_is_the_default_vhost() {
        assert_eq!(Address::parse("amqp://host/%2f").unwrap().vhost, "/");
        assert_eq!(Address::parse("amqp://host/%2F").unwrap().vhost, "/");
    }

    /// A password with an @ in it must not move the host boundary, which is why
    /// the split is on the *last* @ rather than the first.
    #[test]
    fn an_at_sign_in_the_password_does_not_move_the_host() {
        let parsed = Address::parse("amqp://app:p@ss@broker.example/").unwrap();
        assert_eq!(parsed.host, "broker.example");
        assert_eq!(parsed.password, "p@ss");
    }

    #[test]
    fn what_is_not_an_amqp_url_is_refused() {
        assert!(Address::parse("redis://localhost").is_err());
        let tls = Address::parse("amqps://localhost").unwrap_err();
        assert!(tls.contains("TLS is not implemented"), "{tls}");
        assert!(Address::parse("amqp://host:not-a-port/").is_err());
    }

    /// Short strings carry their own length in one octet, so the encoding is
    /// one byte longer than the text and 255 is the ceiling.
    #[test]
    fn a_short_string_carries_its_length() {
        let mut out = Vec::new();
        short_string(&mut out, "queue");
        assert_eq!(out, b"\x05queue");

        let mut long = Vec::new();
        short_string(&mut long, &"x".repeat(300));
        assert_eq!(long.len(), 256);
        assert_eq!(long[0], 255);
    }

    #[test]
    fn a_long_string_carries_four_bytes_of_length() {
        let mut out = Vec::new();
        long_string(&mut out, b"hi");
        assert_eq!(out, b"\x00\x00\x00\x02hi");
    }

    /// Reading back what `publish` writes: the flags say which properties are
    /// there, and they are read in the order the specification fixes rather
    /// than the order they were thought of.
    #[test]
    fn properties_are_read_in_the_specified_order() {
        let mut header = Vec::new();
        header.extend_from_slice(&BASIC.to_be_bytes());
        header.extend_from_slice(&0u16.to_be_bytes());
        header.extend_from_slice(&42u64.to_be_bytes());
        header.extend_from_slice(&((1u16 << 15) | (1 << 12) | (1 << 10) | (1 << 7)).to_be_bytes());
        short_string(&mut header, "application/json");
        header.push(2);
        short_string(&mut header, "corr-1");
        short_string(&mut header, "msg-1");

        let (size, content_type, correlation, message) = read_properties(&header).unwrap();
        assert_eq!(size, 42);
        assert_eq!(content_type, "application/json");
        assert_eq!(correlation, "corr-1");
        assert_eq!(message, "msg-1");
    }

    /// A header with nothing but the body size still parses, and says so with
    /// empty strings rather than by failing.
    #[test]
    fn a_header_with_no_properties_is_still_a_header() {
        let mut header = Vec::new();
        header.extend_from_slice(&BASIC.to_be_bytes());
        header.extend_from_slice(&0u16.to_be_bytes());
        header.extend_from_slice(&0u64.to_be_bytes());
        header.extend_from_slice(&0u16.to_be_bytes());
        assert_eq!(
            read_properties(&header).unwrap(),
            (0, String::new(), String::new(), String::new())
        );
    }

    /// A truncated frame is a stream that has gone out of step, and saying so
    /// is better than reading whatever follows as a value.
    #[test]
    fn a_truncated_value_is_refused() {
        let mut reader = Reader::new(&[0x05, b'q']);
        assert!(reader.short_string().is_err());
        let mut empty = Reader::new(&[]);
        assert!(empty.long_long().is_err());
    }

    #[test]
    fn what_the_broker_said_survives_being_reported() {
        let mut body = Vec::new();
        body.extend_from_slice(&406u16.to_be_bytes());
        short_string(
            &mut body,
            "PRECONDITION_FAILED - inequivalent arg 'durable'",
        );
        let message = closed(&body).unwrap();
        assert!(message.contains("406"), "{message}");
        assert!(message.contains("inequivalent arg 'durable'"), "{message}");
    }
}
