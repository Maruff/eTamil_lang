# AMQP — a message broker, and what "sent" is allowed to mean

AMQP 0-9-1, which is what RabbitMQ speaks. Implemented in `src/amqp.rs` rather than taken from a crate, for the reason `src/redis.rs` gives: the subset a program needs is small, the TCP stack was already there, and the crates that speak AMQP carry an async runtime to put a few hundred bytes down a socket.

No feature flag. It links nothing and costs nothing in a build that never calls it.

## The decision everything follows from

`Basic.Publish` has no reply. Writing it to the socket tells you the bytes left this process, and nothing whatever about whether the broker has them. For a queue carrying payment instructions that is not a useful thing to be told, so two things are always on:

**Publisher confirms.** The channel turns them on at open, and every publish waits for the broker's `Basic.Ack` before answering. `செய்தி_அனுப்பு` returning `சரி` means the broker has taken responsibility for the message.

**Mandatory.** Confirms say the broker *accepted* the message; they do not say it reached a queue. An exchange with nothing bound to the routing key accepts it, discards it, and then acknowledges — so confirms alone answer "yes" for a message that went nowhere. With `mandatory` the broker sends `Basic.Return` first, and that comes back as `தவறு` naming the routing key.

Neither is optional and neither is configurable. A library whose publish can mean three different things depending on a flag is a library where the flag is read once and then forgotten.

Messages are also published **persistent**, because a durable queue holding transient messages loses them on a restart — which is the failure that looks exactly like durability right up until the moment it matters.

## The nine

| Function | What it does |
|---|---|
| `செய்தி_இணை(உரலி)` | `amqp://user:pass@host:port/vhost` — connect, authenticate, open a channel |
| `செய்தி_வரிசை(பெயர், நிலையானதா)` | declare a queue → how many messages are waiting in it |
| `செய்தி_பரிமாற்றி(பெயர், வகை, நிலையானதா)` | declare an exchange: `direct`, `topic`, `fanout`, `headers` |
| `செய்தி_பிணை(வரிசை, பரிமாற்றி, திறவுகோல்)` | bind one to the other |
| `செய்தி_அனுப்பு(பரிமாற்றி, திறவுகோல், உடல், பண்புகள்)` | publish, and wait to be told |
| `செய்தி_பெறு(வரிசை)` | one message, or `இன்மை` if the queue is empty |
| `செய்தி_ஏற்பு(குறிச்சொல்)` | done with it |
| `செய்தி_மறு(குறிச்சொல், மீண்டுமா)` | not done with it |
| `செய்தி_பிரி()` | close politely |

`nUlakam/qaLam/ceyqi.qmz` has the three shapes most programs write on top of those: `வரிசையைத்_தயாரி` (exchange, queue and binding in one call, all durable), `ஜேசானாக_அனுப்பு`, and `ஒன்றைச்_செயலாக்கு`, which is the acknowledgement discipline written once.

## Acknowledging is not automatic, and that is the point

`செய்தி_பெறு` does not acknowledge by taking the message. It stays the broker's responsibility until `செய்தி_ஏற்பு`, so a program that reads one and then fails leaves it to be delivered again — marked `மீண்டும்_வந்ததா`, so the second reader knows. Auto-acknowledging would make `செய்தி_பெறு` a function that can lose a payment instruction by being interrupted at the wrong microsecond.

`செய்தி_மறு` takes `மீண்டுமா`, and the two answers are genuinely different:

- **`மெய்`** puts the message back for another attempt. Right for a failure that will pass later — the database was down. A message that fails *every* time and is always requeued is a loop that never ends.
- **`பொய்`** sends it to the dead-letter exchange if the queue names one, and **discards it** if the queue does not.

Neither is a safe default, so there is no default. The caller chooses, having read that paragraph.

## One connection, one channel

The shape `ரெடிஸ்_இணை` has, for the reason it has it: a channel carries per-channel state — confirm mode, unacknowledged deliveries, prefetch — so handing one to two requests has the hazard of handing round a transaction. Until connections can be named and leased, `செய்தி_இணை` opens its own and holds it for the life of the program.

## What is not here

**No `amqps`.** TLS belongs in this and is not half-written here; an `amqps://` URL is refused with a message saying so rather than silently connecting in the clear. Until it exists, a broker reached across an untrusted network wants a tunnel.

**No `Basic.Consume`** — the push mode, where the broker streams messages at an open socket. `செய்தி_பெறு` pulls one at a time, which is what a program with a request-shaped life can actually use. A consumer loop needs a worker that outlives a request, and that is a design rather than an omission to paper over.

**No transactions** (`tx.select`). Confirms are what RabbitMQ recommends and they are considerably faster; AMQP transactions are not what their name suggests across queues.

## How it is checked

Three layers, and the third is the one that counts.

1. **`src/amqp.rs` unit tests** — URL parsing, the string encodings, the content-header property order.
2. **`tests/amqp_loopback.rs`** — a scripted peer on loopback that the client talks to through a whole conversation, with the frames it sends asserted **byte for byte against expectations written out by hand from the specification's tables**. This catches a field in the wrong order, a bit packed from the wrong end, a length written as a short where the specification says a long. It cannot catch a misreading of the specification, because the same reading is on both sides of that socket.
3. **`tests/amqp.rs`** — against RabbitMQ, which has not read this repository. `#[ignore]`, run with `-- --ignored`, and CI runs a `rabbitmq:3` service container on every push.

```bash
docker run -d -p 5672:5672 rabbitmq:3
cd etamil_compiler && cargo test --test amqp -- --ignored
etamil --vm examples/api/ceyqi_varicY.qmz
```

`ETAMIL_AMQP` points both somewhere other than `amqp://localhost`.
