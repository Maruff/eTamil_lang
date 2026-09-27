// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! The board the VM runs on: pins, time and serial ports for
//! `nUlakam/vaZporuL/vaZporuL.qmz`.
//!
//! Three boards, chosen once, on first use:
//!
//! - **sim** (`ETAMIL_BOARD=sim`): a board in memory, on any machine. Pins
//!   hold what a test sets, time moves only when told, and serial ports are
//!   named queues (`sim:bus`, or a number as an Arduino names its ports). What
//!   lets firmware logic be tested with `etamil --vm` before it is uploaded.
//! - **pi**: a Raspberry Pi, found by its GPIO chip. Pins through the Linux GPIO
//!   character device, by BCM number; serial ports by device path.
//! - **host**: any other machine. Serial ports by device path, on Linux and
//!   macOS; no pins.
//!
//! State lives here, not in the program: an eTamil function cannot change a
//! module's variables, so the board cannot be a value vaZporuL keeps.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Output,
    Input,
    InputPullUp,
}

#[derive(Default)]
struct SimPort {
    /// Lines fed in by போலி_தொடர்_ஊட்டு, not yet read.
    incoming: VecDeque<String>,
    /// Written, not yet ended by a newline.
    partial: String,
    /// Whole lines written since போலி_தொடர்_வெளியீடு last asked.
    lines: Vec<String>,
}

enum Handle {
    Sim(String),
    #[cfg(unix)]
    Device(unix_serial::Port),
}

enum Kind {
    Sim,
    #[cfg(target_os = "linux")]
    Pi(pi_gpio::Chip),
    Host,
}

struct Board {
    kind: Kind,
    /// When the program began, for a real board's millis.
    started: Option<Instant>,
    /// Simulated milliseconds; only the sim board's clock.
    sim_ms: u64,
    modes: HashMap<i64, Mode>,
    levels: HashMap<i64, bool>,
    analogs: HashMap<i64, i64>,
    /// The simulated board's tones: pin to Hz.
    tones: HashMap<i64, i64>,
    sim_ports: HashMap<String, SimPort>,
    handles: Vec<Option<Handle>>,
}

thread_local! {
    static BOARD: RefCell<Option<Board>> = const { RefCell::new(None) };
}

fn with<T>(f: impl FnOnce(&mut Board) -> T) -> T {
    BOARD.with(|cell| {
        let mut slot = cell.borrow_mut();
        let board = slot.get_or_insert_with(|| Board {
            kind: detect(),
            started: None,
            sim_ms: 0,
            modes: HashMap::new(),
            levels: HashMap::new(),
            analogs: HashMap::new(),
            tones: HashMap::new(),
            sim_ports: HashMap::new(),
            handles: Vec::new(),
        });
        f(board)
    })
}

fn detect() -> Kind {
    // In the browser there is no board but a simulated one.
    if cfg!(target_family = "wasm") || std::env::var("ETAMIL_BOARD").map(|b| b == "sim").unwrap_or(false) {
        return Kind::Sim;
    }
    #[cfg(target_os = "linux")]
    if let Some(chip) = pi_gpio::Chip::find() {
        return Kind::Pi(chip);
    }
    Kind::Host
}

const NO_PINS: &str = "இந்தக் கணினியில் முனைகள் இல்லை  (this machine has no pins a program can use: \
                       run it on a Raspberry Pi, or with ETAMIL_BOARD=sim to simulate one)";

fn sim_only(what: &str) -> String {
    format!("{} போலிப் பலகையில் மட்டும்  ({} works only on the simulated board: ETAMIL_BOARD=sim)", what, what)
}

// --- the board --------------------------------------------------------------------

/// "sim", "pi" or "host".
pub fn name() -> &'static str {
    with(|b| match b.kind {
        Kind::Sim => "sim",
        #[cfg(target_os = "linux")]
        Kind::Pi(_) => "pi",
        Kind::Host => "host",
    })
}

// --- pins ---------------------------------------------------------------------------

/// "out", "in" or "in_pullup".
pub fn pin_mode(pin: i64, mode: &str) -> Result<(), String> {
    let mode = match mode {
        "out" => Mode::Output,
        "in" => Mode::Input,
        "in_pullup" => Mode::InputPullUp,
        other => return Err(format!("முனை வகை '{}' இல்லை  (no pin mode '{}': out, in or in_pullup)", other, other)),
    };
    with(|b| {
        match &mut b.kind {
            Kind::Sim => {}
            #[cfg(target_os = "linux")]
            Kind::Pi(chip) => chip.request(pin, mode)?,
            Kind::Host => return Err(NO_PINS.to_string()),
        }
        b.modes.insert(pin, mode);
        if mode == Mode::Output {
            b.levels.insert(pin, false);
        }
        Ok(())
    })
}

fn mode_of(b: &Board, pin: i64) -> Result<Mode, String> {
    b.modes.get(&pin).copied().ok_or_else(|| {
        format!(
            "முனை {} க்கு வகை இல்லை  (pin {} has no mode yet: call முனை_வெளியீடு or முனை_உள்ளீடு first)",
            pin, pin
        )
    })
}

pub fn pin_write(pin: i64, high: bool) -> Result<(), String> {
    with(|b| {
        if mode_of(b, pin)? != Mode::Output {
            return Err(format!("முனை {} வெளியீடு அல்ல  (pin {} is an input: make it an output first)", pin, pin));
        }
        match &mut b.kind {
            Kind::Sim => {}
            #[cfg(target_os = "linux")]
            Kind::Pi(chip) => chip.write(pin, high)?,
            Kind::Host => return Err(NO_PINS.to_string()),
        }
        b.levels.insert(pin, high);
        Ok(())
    })
}

pub fn pin_read(pin: i64) -> Result<bool, String> {
    with(|b| {
        let mode = mode_of(b, pin)?;
        match &mut b.kind {
            // An output reads back what was written; an input what the test
            // set, or what a floating or pulled-up pin rests at.
            Kind::Sim => Ok(b.levels.get(&pin).copied().unwrap_or(mode == Mode::InputPullUp)),
            #[cfg(target_os = "linux")]
            Kind::Pi(chip) => chip.read(pin),
            Kind::Host => Err(NO_PINS.to_string()),
        }
    })
}

/// 0 to 1023.
pub fn analog_read(pin: i64) -> Result<i64, String> {
    with(|b| match b.kind {
        Kind::Sim => Ok(b.analogs.get(&pin).copied().unwrap_or(0)),
        _ => Err(format!(
            "இந்தப் பலகையில் ஒப்புமை உள்ளீடு இல்லை  (this board has no analog inputs; pin {} needs an ADC \
             such as the MCP3008, or an Arduino reading it)",
            pin
        )),
    })
}

/// A tone of `hz` on a pin, or none. Only the simulated board makes one: a Pi
/// has no timer-driven square wave to give a program.
pub fn tone(pin: i64, hz: Option<i64>) -> Result<(), String> {
    with(|b| match b.kind {
        Kind::Sim => {
            match hz {
                Some(hz) => b.tones.insert(pin, hz),
                None => b.tones.remove(&pin),
            };
            Ok(())
        }
        _ => Err(format!(
            "இந்தப் பலகையில் ஒலி இல்லை  (this board makes no tones: on pin {}, drive an active buzzer with முனை_எழுது)",
            pin
        )),
    })
}

// --- time -----------------------------------------------------------------------------

pub fn millis() -> u64 {
    with(|b| match b.kind {
        Kind::Sim => b.sim_ms,
        _ => b.started.get_or_insert_with(Instant::now).elapsed().as_millis() as u64,
    })
}

/// On the sim board a pause moves its clock; anywhere else it waits.
pub fn sleep_ms(ms: u64) {
    let simulated = with(|b| {
        if matches!(b.kind, Kind::Sim) {
            b.sim_ms += ms;
            true
        } else {
            false
        }
    });
    if !simulated {
        std::thread::sleep(Duration::from_millis(ms));
    }
}

// --- serial ports ------------------------------------------------------------------------

/// A simulated port's name: "sim:bus" is "bus", and a number is itself.
fn sim_name(device: &str) -> String {
    device.strip_prefix("sim:").unwrap_or(device).to_string()
}

fn is_number(device: &str) -> bool {
    !device.is_empty() && device.chars().all(|c| c.is_ascii_digit())
}

/// Open a port: its handle, or why not (a தவறு the program handles).
pub fn serial_open(device: &str, baud: u32) -> Result<i64, String> {
    with(|b| {
        let handle = if matches!(b.kind, Kind::Sim) {
            if !device.starts_with("sim:") && !is_number(device) {
                return Err(format!(
                    "துறை {} இல்லை  (no port {} on the simulated board: name one sim:<name>, or by number)",
                    device, device
                ));
            }
            let name = sim_name(device);
            b.sim_ports.entry(name.clone()).or_default();
            Handle::Sim(name)
        } else if device.starts_with("sim:") {
            return Err(sim_only(device));
        } else if is_number(device) {
            return Err(format!(
                "துறை {} ஒரு Arduino எண்  (port {} is how an Arduino numbers its ports; here, name the device, \
                 such as /dev/ttyACM0)",
                device, device
            ));
        } else {
            open_device(device, baud)?
        };
        b.handles.push(Some(handle));
        Ok(b.handles.len() as i64)
    })
}

#[cfg(unix)]
fn open_device(device: &str, baud: u32) -> Result<Handle, String> {
    unix_serial::Port::open(device, baud).map(Handle::Device)
}

#[cfg(not(unix))]
fn open_device(device: &str, _baud: u32) -> Result<Handle, String> {
    Err(format!(
        "துறை {} திறக்க முடியவில்லை  (cannot open {}: the VM's serial ports work on Linux and macOS, \
         such as a Raspberry Pi; on Windows, test with ETAMIL_BOARD=sim)",
        device, device
    ))
}

fn handle_error(handle: i64) -> String {
    format!("துறை {} திறந்திருக்கவில்லை  (port {} is not open)", handle, handle)
}

fn slot(b: &mut Board, handle: i64) -> Result<&mut Handle, String> {
    usize::try_from(handle - 1)
        .ok()
        .and_then(|i| b.handles.get_mut(i))
        .and_then(Option::as_mut)
        .ok_or_else(|| handle_error(handle))
}

/// The next whole line, waiting up to `wait_ms`; `None` when none has come.
pub fn serial_read_line(handle: i64, wait_ms: u64) -> Result<Option<String>, String> {
    // A simulated line has either arrived or not; only a device waits.
    #[cfg(not(unix))]
    let _ = wait_ms;
    with(|b| {
        let port = match slot(b, handle)? {
            Handle::Sim(name) => name.clone(),
            #[cfg(unix)]
            Handle::Device(port) => return port.read_line(wait_ms),
        };
        Ok(b.sim_ports.entry(port).or_default().incoming.pop_front())
    })
}

/// Bytes written.
pub fn serial_write(handle: i64, text: &str) -> Result<usize, String> {
    with(|b| {
        let port = match slot(b, handle)? {
            Handle::Sim(name) => name.clone(),
            #[cfg(unix)]
            Handle::Device(port) => return port.write(text),
        };
        let sim = b.sim_ports.entry(port).or_default();
        sim.partial.push_str(text);
        while let Some(end) = sim.partial.find('\n') {
            let line: String = sim.partial.drain(..=end).collect();
            sim.lines.push(line.trim_end_matches(['\n', '\r']).to_string());
        }
        Ok(text.len())
    })
}

pub fn serial_close(handle: i64) -> Result<(), String> {
    with(|b| {
        slot(b, handle)?;
        b.handles[(handle - 1) as usize] = None;
        Ok(())
    })
}

// --- the simulated board's controls ---------------------------------------------------------

fn sim_board<T>(what: &str, f: impl FnOnce(&mut Board) -> T) -> Result<T, String> {
    with(|b| if matches!(b.kind, Kind::Sim) { Ok(f(b)) } else { Err(sim_only(what)) })
}

pub fn sim_set_pin(pin: i64, high: bool) -> Result<(), String> {
    sim_board("போலி_முனை", |b| {
        b.levels.insert(pin, high);
    })
}

pub fn sim_set_analog(pin: i64, value: i64) -> Result<(), String> {
    if !(0..=1023).contains(&value) {
        return Err(format!("ஒப்புமை அளவு 0–1023  (an analog reading is 0 to 1023, not {})", value));
    }
    sim_board("போலி_ஒப்புமை", |b| {
        b.analogs.insert(pin, value);
    })
}

pub fn sim_advance_ms(ms: u64) -> Result<(), String> {
    sim_board("போலி_நேரம்", |b| b.sim_ms += ms)
}

pub fn sim_feed(device: &str, line: &str) -> Result<(), String> {
    sim_board("போலி_தொடர்_ஊட்டு", |b| {
        let port = b.sim_ports.entry(sim_name(device)).or_default();
        for piece in line.split('\n') {
            port.incoming.push_back(piece.trim_end_matches('\r').to_string());
        }
    })
}

pub fn sim_output(device: &str) -> Result<Vec<String>, String> {
    sim_board("போலி_தொடர்_வெளியீடு", |b| std::mem::take(&mut b.sim_ports.entry(sim_name(device)).or_default().lines))
}

// --- a real serial port, on Linux and macOS ---------------------------------------------------

#[cfg(unix)]
mod unix_serial {
    use std::ffi::CString;
    use std::time::{Duration, Instant};

    pub struct Port {
        fd: libc::c_int,
        name: String,
        pending: Vec<u8>,
    }

    fn speed(baud: u32) -> Option<libc::speed_t> {
        Some(match baud {
            1200 => libc::B1200,
            2400 => libc::B2400,
            4800 => libc::B4800,
            9600 => libc::B9600,
            19200 => libc::B19200,
            38400 => libc::B38400,
            57600 => libc::B57600,
            115200 => libc::B115200,
            230400 => libc::B230400,
            _ => return None,
        })
    }

    fn failed(name: &str, what: &str) -> String {
        format!(
            "துறை {} திறக்க முடியவில்லை  (cannot open {}: {}: {})",
            name,
            name,
            what,
            std::io::Error::last_os_error()
        )
    }

    impl Port {
        pub fn open(name: &str, baud: u32) -> Result<Port, String> {
            let rate = speed(baud).ok_or_else(|| {
                format!("வேகம் {} இல்லை  (no baud rate {}: 1200 to 230400, the usual steps)", baud, baud)
            })?;
            let path = CString::new(name).map_err(|_| failed(name, "a NUL in the name"))?;
            // SAFETY: plain POSIX calls on a descriptor this function owns;
            // every result is checked before the descriptor is used.
            unsafe {
                let fd = libc::open(path.as_ptr(), libc::O_RDWR | libc::O_NOCTTY | libc::O_NONBLOCK);
                if fd < 0 {
                    return Err(failed(name, "open"));
                }
                let mut tty: libc::termios = std::mem::zeroed();
                if libc::tcgetattr(fd, &mut tty) != 0 {
                    let why = failed(name, "not a serial port");
                    libc::close(fd);
                    return Err(why);
                }
                libc::cfmakeraw(&mut tty);
                tty.c_cflag |= libc::CLOCAL | libc::CREAD;
                tty.c_cc[libc::VMIN] = 0;
                tty.c_cc[libc::VTIME] = 0;
                libc::cfsetispeed(&mut tty, rate);
                libc::cfsetospeed(&mut tty, rate);
                if libc::tcsetattr(fd, libc::TCSANOW, &tty) != 0 {
                    let why = failed(name, "setting it up");
                    libc::close(fd);
                    return Err(why);
                }
                // Blocking writes; reads wait in poll(), never in read().
                let flags = libc::fcntl(fd, libc::F_GETFL);
                libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK);
                Ok(Port { fd, name: name.to_string(), pending: Vec::new() })
            }
        }

        fn take_line(&mut self) -> Option<String> {
            let end = self.pending.iter().position(|&b| b == b'\n')?;
            let mut line: Vec<u8> = self.pending.drain(..=end).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            Some(String::from_utf8_lossy(&line).into_owned())
        }

        pub fn read_line(&mut self, wait_ms: u64) -> Result<Option<String>, String> {
            let deadline = Instant::now() + Duration::from_millis(wait_ms);
            loop {
                if let Some(line) = self.take_line() {
                    return Ok(Some(line));
                }
                let left = deadline.saturating_duration_since(Instant::now()).as_millis() as libc::c_int;
                let mut poll = libc::pollfd { fd: self.fd, events: libc::POLLIN, revents: 0 };
                // SAFETY: one pollfd for a descriptor this Port owns.
                let ready = unsafe { libc::poll(&mut poll, 1, left) };
                if ready < 0 {
                    return Err(format!("துறை {} படிக்க முடியவில்லை  (cannot read {}: {})", self.name, self.name, std::io::Error::last_os_error()));
                }
                if ready == 0 {
                    return Ok(None);
                }
                let mut buffer = [0u8; 256];
                // SAFETY: reads into a local buffer of the length given.
                let n = unsafe { libc::read(self.fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len()) };
                if n < 0 {
                    return Err(format!("துறை {} படிக்க முடியவில்லை  (cannot read {}: {})", self.name, self.name, std::io::Error::last_os_error()));
                }
                if n == 0 && poll.revents & libc::POLLHUP != 0 {
                    return Err(format!("துறை {} துண்டிக்கப்பட்டது  ({} was unplugged)", self.name, self.name));
                }
                self.pending.extend_from_slice(&buffer[..n as usize]);
            }
        }

        pub fn write(&mut self, text: &str) -> Result<usize, String> {
            let mut bytes = text.as_bytes();
            let total = bytes.len();
            while !bytes.is_empty() {
                // SAFETY: writes from a slice of the length given.
                let n = unsafe { libc::write(self.fd, bytes.as_ptr() as *const libc::c_void, bytes.len()) };
                if n <= 0 {
                    return Err(format!("துறை {} எழுத முடியவில்லை  (cannot write {}: {})", self.name, self.name, std::io::Error::last_os_error()));
                }
                bytes = &bytes[n as usize..];
            }
            Ok(total)
        }
    }

    impl Drop for Port {
        fn drop(&mut self) {
            // SAFETY: the descriptor is this Port's, closed once.
            unsafe {
                libc::close(self.fd);
            }
        }
    }
}

// --- a Raspberry Pi's pins, through the GPIO character device -------------------------------------

#[cfg(target_os = "linux")]
mod pi_gpio {
    //! Linux's GPIO v2 interface, `/dev/gpiochipN`: one line request per pin.
    //! Not sysfs, which the Pi 5's kernels number differently and which is
    //! going away.
    use super::Mode;
    use std::collections::HashMap;
    use std::ffi::CString;

    /// The chips whose lines are the 40-pin header's BCM GPIOs.
    const LABELS: &[&str] = &["pinctrl-bcm2711", "pinctrl-bcm2835", "pinctrl-bcm2712", "pinctrl-rp1"];

    #[repr(C)]
    struct ChipInfo {
        name: [u8; 32],
        label: [u8; 32],
        lines: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct LineAttribute {
        id: u32,
        padding: u32,
        value: u64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ConfigAttribute {
        attr: LineAttribute,
        mask: u64,
    }

    #[repr(C)]
    struct LineConfig {
        flags: u64,
        num_attrs: u32,
        padding: [u32; 5],
        attrs: [ConfigAttribute; 10],
    }

    #[repr(C)]
    struct LineRequest {
        offsets: [u32; 64],
        consumer: [u8; 32],
        config: LineConfig,
        num_lines: u32,
        event_buffer_size: u32,
        padding: [u32; 5],
        fd: i32,
    }

    #[repr(C)]
    struct LineValues {
        bits: u64,
        mask: u64,
    }

    const fn ioc(dir: u64, nr: u64, size: usize) -> u64 {
        (dir << 30) | ((size as u64) << 16) | (0xB4 << 8) | nr
    }
    const GET_CHIPINFO: u64 = ioc(2, 0x01, std::mem::size_of::<ChipInfo>());
    const GET_LINE: u64 = ioc(3, 0x07, std::mem::size_of::<LineRequest>());
    const GET_VALUES: u64 = ioc(3, 0x0E, std::mem::size_of::<LineValues>());
    const SET_VALUES: u64 = ioc(3, 0x0F, std::mem::size_of::<LineValues>());

    const FLAG_INPUT: u64 = 1 << 2;
    const FLAG_OUTPUT: u64 = 1 << 3;
    const FLAG_BIAS_PULL_UP: u64 = 1 << 8;
    const FLAG_BIAS_DISABLED: u64 = 1 << 10;

    pub struct Chip {
        fd: libc::c_int,
        lines: u32,
        /// Each requested pin's line descriptor.
        pins: HashMap<i64, libc::c_int>,
    }

    fn os_error(what: &str, pin: i64) -> String {
        format!("முனை {}: {}  (pin {}: {}: {})", pin, what, pin, what, std::io::Error::last_os_error())
    }

    impl Chip {
        pub fn find() -> Option<Chip> {
            for n in 0..16 {
                let path = CString::new(format!("/dev/gpiochip{}", n)).ok()?;
                // SAFETY: open, one ioctl into a local struct, close on mismatch.
                unsafe {
                    let fd = libc::open(path.as_ptr(), libc::O_RDWR | libc::O_CLOEXEC);
                    if fd < 0 {
                        continue;
                    }
                    let mut info: ChipInfo = std::mem::zeroed();
                    if libc::ioctl(fd, GET_CHIPINFO as _, &mut info) == 0 {
                        let label = String::from_utf8_lossy(&info.label);
                        let label = label.trim_end_matches('\0');
                        if LABELS.contains(&label) {
                            return Some(Chip { fd, lines: info.lines, pins: HashMap::new() });
                        }
                    }
                    libc::close(fd);
                }
            }
            None
        }

        pub fn request(&mut self, pin: i64, mode: Mode) -> Result<(), String> {
            if pin < 0 || pin >= self.lines as i64 || pin > 27 {
                return Err(format!("முனை {} இல்லை  (no GPIO {}: a Pi's header has GPIO 0 to 27)", pin, pin));
            }
            if let Some(old) = self.pins.remove(&pin) {
                // SAFETY: the line descriptor is ours, closed once.
                unsafe { libc::close(old) };
            }
            // SAFETY: a zeroed request filled in field by field, as the kernel's
            // header defines it; the ioctl result is checked.
            unsafe {
                let mut request: LineRequest = std::mem::zeroed();
                request.offsets[0] = pin as u32;
                request.num_lines = 1;
                let consumer = b"etamil";
                request.consumer[..consumer.len()].copy_from_slice(consumer);
                request.config.flags = match mode {
                    Mode::Output => FLAG_OUTPUT,
                    Mode::Input => FLAG_INPUT | FLAG_BIAS_DISABLED,
                    Mode::InputPullUp => FLAG_INPUT | FLAG_BIAS_PULL_UP,
                };
                if libc::ioctl(self.fd, GET_LINE as _, &mut request) != 0 {
                    return Err(os_error("கேட்க முடியவில்லை (could not claim it)", pin));
                }
                self.pins.insert(pin, request.fd);
            }
            Ok(())
        }

        fn line(&self, pin: i64) -> Result<libc::c_int, String> {
            self.pins.get(&pin).copied().ok_or_else(|| format!("முனை {} கேட்கப்படவில்லை  (pin {} was not set up)", pin, pin))
        }

        pub fn write(&mut self, pin: i64, high: bool) -> Result<(), String> {
            let fd = self.line(pin)?;
            let mut values = LineValues { bits: u64::from(high), mask: 1 };
            // SAFETY: one ioctl on our line descriptor with a local struct.
            if unsafe { libc::ioctl(fd, SET_VALUES as _, &mut values) } != 0 {
                return Err(os_error("எழுத முடியவில்லை (could not drive it)", pin));
            }
            Ok(())
        }

        pub fn read(&mut self, pin: i64) -> Result<bool, String> {
            let fd = self.line(pin)?;
            let mut values = LineValues { bits: 0, mask: 1 };
            // SAFETY: as in write.
            if unsafe { libc::ioctl(fd, GET_VALUES as _, &mut values) } != 0 {
                return Err(os_error("படிக்க முடியவில்லை (could not read it)", pin));
            }
            Ok(values.bits & 1 == 1)
        }
    }

    impl Drop for Chip {
        fn drop(&mut self) {
            // SAFETY: every descriptor here is ours, closed once.
            unsafe {
                for fd in self.pins.values() {
                    libc::close(*fd);
                }
                libc::close(self.fd);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_kernel_s_structures_are_its_sizes() {
            // linux/gpio.h: struct gpio_v2_line_request is 592 bytes.
            assert_eq!(std::mem::size_of::<LineRequest>(), 592);
            assert_eq!(std::mem::size_of::<ChipInfo>(), 68);
            assert_eq!(GET_LINE, 0xC250_B407);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sim() {
        with(|_| ());
        BOARD.with(|cell| cell.borrow_mut().as_mut().expect("made").kind = Kind::Sim);
    }

    #[test]
    fn the_simulated_board_holds_what_it_is_told() {
        sim();
        assert_eq!(name(), "sim");
        pin_mode(13, "out").unwrap();
        pin_write(13, true).unwrap();
        assert!(pin_read(13).unwrap());
        pin_mode(3, "in_pullup").unwrap();
        assert!(pin_read(3).unwrap());
        sim_set_pin(3, false).unwrap();
        assert!(!pin_read(3).unwrap());
        assert!(pin_write(3, true).is_err(), "an input is not driven");
        assert!(pin_read(7).is_err(), "a pin with no mode");
        sleep_ms(40);
        sim_advance_ms(60).unwrap();
        assert_eq!(millis(), 100);

        let bus = serial_open("1", 19200).unwrap();
        assert_eq!(serial_read_line(bus, 0).unwrap(), None);
        sim_feed("1", ">03,P*7F").unwrap();
        assert_eq!(serial_read_line(bus, 0).unwrap().as_deref(), Some(">03,P*7F"));
        serial_write(bus, "<03,").unwrap();
        serial_write(bus, "3:N:250*03\n").unwrap();
        assert_eq!(sim_output("1").unwrap(), vec!["<03,3:N:250*03"]);
        serial_close(bus).unwrap();
        assert!(serial_write(bus, "x").is_err());
        assert!(serial_open("/dev/ttyACM0", 9600).is_err(), "no real devices on the sim board");
    }
}
