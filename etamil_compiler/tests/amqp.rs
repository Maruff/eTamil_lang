//! AMQP against a real broker.
//!
//! `tests/amqp_loopback.rs` checks that the client puts the specification's
//! bytes on the wire, against a peer this repository also wrote. That catches a
//! great deal and cannot catch the one thing that matters most: a misreading of
//! the specification, which would sit on both sides of that socket and agree
//! with itself. This is the test that cannot be fooled that way, because the
//! thing on the other end is RabbitMQ.
//!
//! `#[ignore]`, and run with `-- --ignored` when a broker is there:
//!
//!     cargo test --test amqp -- --ignored
//!
//! Not an `if env::var(...) { return }` gate, for the reason `tests/pkcs11.rs`
//! gives: a test that returns early is counted as passing, and five green ticks
//! over code that never ran is worse than a red one.
//!
//! `ETAMIL_AMQP` points it somewhere other than a local broker. CI runs a
//! `rabbitmq:3` service container, so this runs for real on every push.

#![cfg(not(target_family = "wasm"))]

use etamil_compiler::amqp::{Connection, Published};

fn broker() -> Connection {
    let url = std::env::var("ETAMIL_AMQP").unwrap_or_else(|_| "amqp://localhost".to_string());
    Connection::open(&url)
        .unwrap_or_else(|why| panic!("no broker at {url}: {why}\nCI runs rabbitmq:3 as a service"))
}

/// A name no other run will collide with, so the tests do not have to be run in
/// order or on an empty broker.
fn unique(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!(
        "etamil-{prefix}-{}-{}",
        std::process::id(),
        now.subsec_nanos()
    )
}

#[test]
#[ignore = "needs a broker: run with -- --ignored"]
fn a_message_published_comes_back_the_same() {
    let mut connection = broker();
    let queue = unique("round-trip");
    assert_eq!(connection.declare_queue(&queue, true).unwrap(), 0);

    let body = br#"{"irn":"a1b2c3","total":"1180.00"}"#;
    assert_eq!(
        connection
            .publish("", &queue, body, "application/json", "corr-1", "msg-1")
            .unwrap(),
        Published::Confirmed
    );

    let delivery = connection.get(&queue).unwrap().expect("the message");
    assert_eq!(delivery.body, body);
    assert_eq!(delivery.content_type, "application/json");
    assert_eq!(delivery.correlation_id, "corr-1");
    assert_eq!(delivery.message_id, "msg-1");
    assert_eq!(delivery.routing_key, queue);
    assert!(!delivery.redelivered);

    connection.ack(delivery.tag).unwrap();
    assert_eq!(connection.get(&queue).unwrap(), None, "and it is gone");
    connection.close().unwrap();
}

/// The property publisher confirms exist for. Without them a publish reports
/// success as soon as the bytes leave, which says nothing about the broker.
#[test]
#[ignore = "needs a broker: run with -- --ignored"]
fn the_broker_says_it_has_the_message_before_publish_returns() {
    let mut connection = broker();
    let queue = unique("confirms");
    connection.declare_queue(&queue, true).unwrap();

    for n in 0..5 {
        assert_eq!(
            connection
                .publish("", &queue, format!("{n}").as_bytes(), "", "", "")
                .unwrap(),
            Published::Confirmed,
            "publish {n}"
        );
    }

    // Confirmed means the broker has them, so a fresh declare must agree. If
    // confirms were being misread, this is where the count would be short.
    let mut second = broker();
    assert_eq!(second.declare_queue(&queue, true).unwrap(), 5);
    second.close().unwrap();
    connection.close().unwrap();
}

/// The other half of the question, and the one confirms alone get wrong. An
/// exchange with nothing bound to the key accepts the message, discards it, and
/// acknowledges — so without `mandatory` this would look like a success.
#[test]
#[ignore = "needs a broker: run with -- --ignored"]
fn a_message_that_routes_nowhere_is_not_a_success() {
    let mut connection = broker();
    let exchange = unique("nowhere");
    connection
        .declare_exchange(&exchange, "direct", true)
        .unwrap();

    match connection
        .publish(&exchange, "nobody.listening", b"x", "", "", "")
        .unwrap()
    {
        Published::Returned { code, text } => {
            assert_eq!(code, 312, "NO_ROUTE is 312");
            assert!(text.contains("NO_ROUTE"), "{text}");
        }
        other => panic!("an unroutable message reported {other:?}"),
    }
    connection.close().unwrap();
}

#[test]
#[ignore = "needs a broker: run with -- --ignored"]
fn a_binding_is_what_makes_a_routing_key_reach_a_queue() {
    let mut connection = broker();
    let exchange = unique("orders");
    let queue = unique("paid");
    connection
        .declare_exchange(&exchange, "direct", true)
        .unwrap();
    connection.declare_queue(&queue, true).unwrap();
    connection.bind(&queue, &exchange, "paid").unwrap();

    assert_eq!(
        connection
            .publish(&exchange, "paid", b"one", "", "", "")
            .unwrap(),
        Published::Confirmed
    );
    let delivery = connection.get(&queue).unwrap().expect("the bound message");
    assert_eq!(delivery.body, b"one");
    assert_eq!(delivery.exchange, exchange);
    connection.ack(delivery.tag).unwrap();

    // A different key on the same exchange reaches nothing, which is the
    // binding doing its job rather than the queue being empty by luck.
    assert!(matches!(
        connection
            .publish(&exchange, "refunded", b"two", "", "", "")
            .unwrap(),
        Published::Returned { .. }
    ));
    connection.close().unwrap();
}

/// Not acknowledging is what makes a crash survivable: the message is still the
/// broker's until someone says otherwise, and comes back marked as redelivered.
#[test]
#[ignore = "needs a broker: run with -- --ignored"]
fn a_rejected_message_comes_back_when_it_is_requeued() {
    let mut connection = broker();
    let queue = unique("requeue");
    connection.declare_queue(&queue, true).unwrap();
    connection.publish("", &queue, b"work", "", "", "").unwrap();

    let first = connection.get(&queue).unwrap().expect("the message");
    assert!(!first.redelivered);
    connection.nack(first.tag, true).unwrap();

    let again = connection.get(&queue).unwrap().expect("it came back");
    assert_eq!(again.body, b"work");
    assert!(again.redelivered, "and it is marked as having been seen");

    // Rejected without requeue, and with no dead-letter exchange on the queue,
    // it is discarded. That is the behaviour, and it is why ஒன்றைச்_செயலாக்கு
    // makes the caller choose rather than picking a default.
    connection.nack(again.tag, false).unwrap();
    assert_eq!(connection.get(&queue).unwrap(), None);
    connection.close().unwrap();
}

/// A body larger than one frame is several, and a client that assumed one would
/// truncate it — silently, because the header says how long it should be.
#[test]
#[ignore = "needs a broker: run with -- --ignored"]
fn a_body_larger_than_a_frame_survives_the_journey() {
    let mut connection = broker();
    let queue = unique("large");
    connection.declare_queue(&queue, true).unwrap();

    let body: Vec<u8> = (0..500_000u32).map(|n| (n % 251) as u8).collect();
    assert_eq!(
        connection.publish("", &queue, &body, "", "", "").unwrap(),
        Published::Confirmed
    );

    let delivery = connection.get(&queue).unwrap().expect("the message");
    assert_eq!(delivery.body.len(), body.len());
    assert_eq!(delivery.body, body, "the body came back changed");
    connection.ack(delivery.tag).unwrap();
    connection.close().unwrap();
}

/// A zero-length message is a message. It has no body frames at all, and a
/// client that waited for one would wait forever.
#[test]
#[ignore = "needs a broker: run with -- --ignored"]
fn an_empty_body_is_still_a_message() {
    let mut connection = broker();
    let queue = unique("empty-body");
    connection.declare_queue(&queue, true).unwrap();
    connection.publish("", &queue, b"", "", "", "").unwrap();

    let delivery = connection
        .get(&queue)
        .unwrap()
        .expect("a message with no body");
    assert_eq!(delivery.body, b"");
    connection.ack(delivery.tag).unwrap();
    connection.close().unwrap();
}

/// What the broker says when it refuses has to arrive intact. Redeclaring a
/// queue with different durability is the refusal everyone meets first.
#[test]
#[ignore = "needs a broker: run with -- --ignored"]
fn a_refusal_arrives_with_the_brokers_own_words() {
    let mut connection = broker();
    let queue = unique("durability");
    connection.declare_queue(&queue, true).unwrap();
    connection.close().unwrap();

    let mut second = broker();
    let why = second.declare_queue(&queue, false).unwrap_err();
    assert!(why.contains("406"), "{why}");
    assert!(why.contains("durable"), "{why}");
}

/// Credentials that are wrong are refused at the handshake, and the message has
/// to say that rather than reporting a broken socket.
#[test]
#[ignore = "needs a broker: run with -- --ignored"]
fn a_wrong_password_does_not_open_a_connection() {
    let base = std::env::var("ETAMIL_AMQP").unwrap_or_else(|_| "amqp://localhost".to_string());
    let host = base.trim_start_matches("amqp://");
    let host = host.rsplit('@').next().unwrap_or(host);
    assert!(Connection::open(&format!("amqp://guest:wrong@{host}")).is_err());
}
