# The eTamil droplet

`201.79.9.90` — DigitalOcean, 2 vCPU, 3.8 GiB RAM, 77 G disk, Ubuntu 24.04.5 LTS.
Hostname `eTamil-paRY`.

Every figure below was measured on the machine, not estimated. Where something
is untested it says so.

---

## What it runs

| | |
|---|---|
| **Nginx** | reverse proxy and TLS for the demo domains |
| **PostgreSQL 16** | tuned for a small box: `shared_buffers=128MB`, `max_connections=20`, `work_mem=4MB` |
| **eTamil 1.1.0** | `/root/.local/bin/etamil`, from the release tarball — **includes the PostgreSQL driver**. `install.sh` without `sudo` puts it under `$HOME`, not `/usr/local/bin`, and nothing adds `~/.local/bin` to `PATH`: reach it by full path. |
| **Ollama** | `granite3.3:2b` (1.5 G) and `llama3.2:3b` (2.0 G) |
| **Toolchains** | Rust 1.98.1, LLVM 18.1.3, Python 3.12.3 + pip, Node 22.23.3 + npm 10.9.9 |
| **Hardening** | UFW (22/80/443), fail2ban, unattended-upgrades, 4 GiB swap at `vm.swappiness=10` |

### What it deliberately does not run

- **etamil.in** — GitHub Pages. Free, CDN-backed, zero server RAM.
- **The browser editor** — WebAssembly in the visitor's browser. Costs the
  droplet nothing at any traffic level.
- **Code generation** — needs a 14B model and 16 GB. See
  [MVP.md](MVP.md#measured-on-the-droplet-2026-09-25).

---

## Memory budget

The whole serving stack is 482 MB. The model is the only large consumer.

| state | used | free |
|---|---|---|
| serving — Nginx, PostgreSQL, four apps | **482 MB** | 3.4 G |
| serving + a model answering | 2,304 MB | 1.6 G |
| serving + an LLVM build | 2,026 MB | 1.9 G |
| **serving + model + build** | **3,848 MB** | **67 MB — do not** |

Either a model or a build fits comfortably. Both at once does not.

---

## Stopping Ollama for a build

`OLLAMA_KEEP_ALIVE=30s` is configured, so a model unloads 30 seconds after the
last request and an idle box usually has the RAM already. Stop it explicitly
for a long build, where someone might use the assistant halfway through.

```bash
systemctl stop ollama
cd /srv/etamil/eTamil/etamil_compiler
LLVM_SYS_180_PREFIX=/usr/lib/llvm-18 cargo build --release --features llvm -j2
systemctl start ollama
```

One line, from your own machine:

```bash
ssh root@201.79.9.90 'bash -lc "systemctl stop ollama && cd /srv/etamil/eTamil/etamil_compiler && LLVM_SYS_180_PREFIX=/usr/lib/llvm-18 cargo build --release --features llvm -j2; systemctl start ollama"'
```

The `;` before the restart rather than `&&` is deliberate: Ollama comes back
whether the build succeeded or failed.

`bash -lc` is not decoration. A non-interactive `ssh` command gets the default
`PATH`, which does not include `/root/.cargo/bin`, so the same line without it
stops at `cargo: command not found` — after Ollama has already been stopped.

**Nothing needs stopping in the other direction.** Builds are occasional and
manual; if one is running when a request arrives, the model simply loads more
slowly. There is no rule to remember for using the AI.

### Measured build times

Both from a clean clone, `-j2`, Ollama stopped:

| build | time | peak memory |
|---|---|---|
| `cargo build --release` | 471 s | 1,643 MB |
| `cargo build --release --features llvm` | 229 s | 1,544 MB |

Swap was not touched. The LLVM build is faster only because it reuses the
first build's dependencies.

**The release binary is the one to deploy**, not a source build: `cargo build
--release` omits the PostgreSQL driver, and a program using `தளம்_இணை
போச்குரசீகுல்` then fails at its first query with *"this build has no
PostgreSQL support"*. The tarball from GitHub releases has it. Verified on
this machine.

---

## Domains

| domain | state |
|---|---|
| `qos.ae` | ✅ HTTPS, placeholder page, certificate to 2026-12-24 |
| `kelir.org` | ✅ HTTPS, placeholder page, certificate to 2026-12-24 |
| `conf.ae` | ✅ HTTPS, placeholder page, certificate to 2026-12-24 — covers `www.conf.ae` |
| `api.kelir.org` | resolves here; awaits the kElir API deploy |
| `ineo.in` | hosted elsewhere, to be moved |

Placeholders are static files at `/var/www/<name>/index.html`, served by nginx
directly — no application, no port, no service. To serve a real site, replace
the `location` block in `/etc/nginx/sites-available/<name>` with a
`proxy_pass`; the header comment in each file shows how.

`configure-domains.sh` in this directory does all of it and is safe to re-run:
it skips any domain that does not resolve to this machine, and leaves existing
certificates alone.

### conf.ae took three attempts

The first two certbot runs failed against a stale resolver, not against this
machine:

```
Invalid response from http://conf.ae/... 194.39.149.163: 404
```

All four authoritative nameservers returned `201.79.9.90`, as did Google,
Cloudflare, Quad9 and OpenDNS. Only Let's Encrypt's own resolver still held the
previous host, and the record's TTL was 7200 s. The third attempt, made after
that expired, succeeded for both `conf.ae` and `www.conf.ae`.

The lesson generalises to any domain moved here: check the TTL, wait it out,
and **prove the move with a staging dry run before spending a real attempt.**
Let's Encrypt allows five failed validations per domain per hour; a dry run
costs none of them, because it runs against the staging CA:

```bash
certbot certonly --nginx -d conf.ae -d www.conf.ae --dry-run --non-interactive
```

---

## What the AI can and cannot do here

Measured, on the six-task and comprehension benchmarks. Harnesses are under
`/root/` on the droplet.

| capability | result |
|---|---|
| Explain a compiler error | ✅ **5/5** |
| Summarise what a program does | ✅ **5/5** |
| Find the right `nUlakam` function | ⚠️ works **when retrieval shortlists it** — the model chose correctly 3/3 |
| Predict what a program prints | ❌ **1/7** |
| Write new eTamil from English | ❌ **2/6** |

Two things follow.

**Retrieval is the bottleneck, not the model.** Recall over the 604 `nUlakam`
functions was 4/8 by keyword and 4/8 by embedding, because 97 functions have no
English doc comment and the rest are terse — `ரூபாய்` reads "the same, with the
rupee sign". Nothing can retrieve what is not described. Writing one English
line per function fixes this without touching the model.

**Generation needs 14B.** `qwen2.5:14b` scored 18/20 on a 16 GB box; nine
models inside 4 GB topped out at 2/6. That is a hardware decision, not a
prompt-engineering one.

The two failures matter less than they look: the Studio can simply *run* a
program to show its output, and the compiler already reports its own errors.
The model's job is explaining them.

---

## Services

All enabled at boot:

```
nginx  postgresql  ollama  fail2ban  unattended-upgrades
```

`unattended-upgrades` **will reboot the machine** when a package requires it —
it did so during setup, taking `libc6`. If an unannounced reboot is
unacceptable once the demo apps carry traffic, set
`Unattended-Upgrade::Automatic-Reboot "false";` in
`/etc/apt/apt.conf.d/50unattended-upgrades` and watch
`/var/run/reboot-required` instead.
