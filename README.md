# wattmeter

[![CI](https://github.com/erwins-enkel/wattmeter/actions/workflows/ci.yml/badge.svg)](https://github.com/erwins-enkel/wattmeter/actions/workflows/ci.yml)

A live terminal chart of laptop power draw, in braille. `btop`'s visual language, but for
the one number `btop` doesn't graph: watts off the battery.

![wattmeter](https://github.com/erwins-enkel/wattmeter/raw/main/docs/demo.gif)

That is a real recording, not a mock-up. It was taken on mains, which is why the live
figure reads `charging at` and the chart stops short of the right edge — charging is not
draw, so those samples are excluded rather than bridged with a plausible-looking line.
See [How it measures](#how-it-measures).

<details>
<summary>Same thing as text, for reading in a terminal</summary>

```
┌ wattmeter BAT1 charging 95% ─────────────────────────────────────────────────────────┐
│charging at 23.42 W   median 6.16 W   mean 6.09 W   p90 6.83 W   used 4.31 Wh over 42m│
│min 5.04 W   peak 8.18 W   pack 73.5 Wh   full-pack at median 11h 56m                 │
└──────────────────────────────────────────────────────────────────────────────────────┘
┌ battery draw last 1h · 85 discharging samples ───────────────────────────────────────┐
│10                                                                                    │
│                                                                                      │
│                     ⢀                    ⢰⣶⡆                                         │
│   ⢸⡄        ⢸⣿⡇    ⢸⣿⣷⣄⣠⣄              ⢀⡀⢸⣿⡇ ⣀⣄                                      │
│   ⢸⣷⣤⣶⣧⣶⣤⣤⣤⣴⣿⣿⣇  ⢠⣶⣼⣿⣿⣿⣿⣿ ⢀    ⢀⣤⣴⣾⣷⡄ ⣤⣾⣿⣼⣿⣷⣶⣿⣿⣶⣦⣤⣾⣦⢠⡀      ⢀⡄                       │
│   ⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣶⣿⣿⣿⣿⣿⣿⣿⣿⣿⣾⣿⣦⣤⣴⣿⣿⣿⣿⣿⣷⣼⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣤⣄⣀⣾⣆⣰⣿⡇                       │
│ 5 ⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇                       │
│   ⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇                       │
│   ⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇                       │
│   ⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇                       │
│   ⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇                       │
│ 0 ⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇                       │
│   -1h                                    -30m                                     now│
└──────────────────────────────────────────────────────────────────────────────────────┘
 1 15m   2 1h   3 3h   4 12h   q quit
```

</details>

The fill runs green at idle to red at the peak, in [Catppuccin](https://catppuccin.com).

Recorded with [vhs](https://github.com/charmbracelet/vhs): `vhs docs/demo.tape`.

## Install

Linux only — it reads `/sys/class/power_supply` and talks to UPower over D-Bus.
Needs Rust 1.88 or newer, which CI compiles against on every push.

Prebuilt x86-64 and aarch64 binaries are attached to each
[release](https://github.com/erwins-enkel/wattmeter/releases), with checksums. Or
build it yourself:

```sh
cargo install --git https://github.com/erwins-enkel/wattmeter
```

That drops the binary in `~/.cargo/bin`, which is on your `PATH` if you installed Rust
through rustup — but not if you installed it from a distro package. Check with
`command -v wattmeter`, and add it yourself if it comes back empty.

Or from a clone:

```sh
cargo build --release && ./target/release/wattmeter
```

## Use

```
wattmeter                       # first battery, last hour
wattmeter --range 12h           # start wider
wattmeter --battery BAT0        # pick a pack
wattmeter --list-batteries      # what this machine exposes
wattmeter --json                # one snapshot for a status bar, then exit
```

`--json` prints the live reading, the window statistics, energy drawn and how much of the
window that energy actually covers. `draw_watts` and `charge_watts` are separate keys and
only one is ever non-null, so a consumer cannot mistake a charge rate for a draw.

`1` `2` `3` `4` switch between 15m / 1h / 3h / 12h. `q`, `Esc` or `ctrl-c` quits.

| Flag | Default | |
|---|---|---|
| `--battery <NAME>` | first readable | Machines with two packs chart one; the header names it |
| `--range <15m\|1h\|3h\|12h>` | `1h` | Range at startup |
| `--interval <SECS>` | `1` | See the note on resolution below |
| `--theme <latte\|frappe\|macchiato\|mocha>` | `mocha` | Catppuccin flavour |
| `--marker <braille\|half-block\|block\|dot>` | `braille` | Braille needs a font with the Braille Patterns block |
| `--color <auto\|truecolor\|ansi>` | `auto` | `auto` reads `COLORTERM`; `ansi` follows your terminal's own palette |
| `--rapl <auto\|on\|off>` | `auto` | CPU panel. `on` refuses to start if the counters are unreadable, and says why |
| `--json` | off | One snapshot to stdout, then exit. Waits one interval when the CPU panel is on |

## Development

Needs [just](https://github.com/casey/just): `cargo install just`, or from your package
manager — `pacman -S just`, `brew install just`, `apt install just`.

```sh
just         # list recipes
just ci      # fmt, clippy (warnings are errors), tests, release build — what CI runs
just msrv    # compile against the declared minimum Rust version
just run     # build and launch
just install # install this working copy to ~/.cargo/bin
just demo    # re-record docs/demo.gif, needs vhs and ttyd
```

CI invokes the same recipes, so a green `just ci` locally means a green pipeline.

## How it measures

Most of the work here is in *not* lying about the data.

**Watts.** `power_now` where the firmware exposes it, otherwise
`current_now × voltage_now`. The sign is ignored — some firmware keeps `current_now`
positive while charging, so direction comes from `status`, never from the sign.

**Resolution.** The embedded controller refreshes about once a second. Polling faster
returns the same reading again rather than a new one, so 1 s is the honest floor, and the
default.

**History.** UPower already logs rate history, so the chart is populated at launch instead
of starting empty — one `GetHistory` call over the system bus, no collector daemon and no
privileges. Its `rate == 0.0` samples are artifacts around AC transitions and are dropped.

**Gaps are gaps.** UPower samples about every second under load and every thirty at rest.
Anything over two minutes breaks the line rather than interpolating across it, so a
three-hour suspend reads as absence — not as a straight line between the readings either
side of it.

**Charging is not draw.** On AC the same counters measure energy going *into* the pack.
Those samples are excluded from the chart and every statistic, and the live figure
relabels itself `charging at`. Time on AC shows up as the gap it is.

**`used`** integrates power over time within each gap-free stretch, trapezoidally, and
reports how many seconds of real sampling are behind the figure. Gaps contribute nothing:
an hour-wide window holding twenty minutes on mains has forty minutes of evidence in it,
and bridging the gap would manufacture energy nobody drew.

**`full-pack at median`** is what a full pack would give you at the median draw. It is a
benchmark figure and deliberately ignores the current charge level — at 50% you do not
have that long left.

Battery draw is whole-system: SoC, display, radios, everything. It is not CPU package
power, and the two should not be read as the same number — which is why the CPU panel is
a separate chart with its own scale rather than a second line on the same axes. The two
share a time axis and a left gutter, so a spike lines up against a spike.

**CPU power** comes from the RAPL energy counters, which are accumulators rather than
power readings: watts are the difference between two samples over the time between them.
The counter wraps, and also resets across suspend — a reset is indistinguishable from a
wrap, so intervals longer than ten seconds are discarded rather than turned into an
invented spike. RAPL keeps no history, so unlike the battery chart this one starts empty
and fills as you watch.

## CPU power is off unless you allow it

`/sys/class/powercap/*/energy_uj` is root-only, and that is deliberate. Fine-grained power
readings are a side channel: [PLATYPUS](https://platypusattack.com) (CVE-2020-8694) used
unprivileged RAPL access to recover AES-NI keys and defeat KASLR, and the kernel locked
the counters down in response.

So this is a trade, and it is yours to make rather than mine to make quietly. `wattmeter`
reads the counters if it can and says nothing if it cannot. When it can, a second panel
appears below the battery chart:

```
┌ cpu package-0 · core 4.78 W · uncore 1.31 W · dram 0.83 W · platform psys 26.30 W ───────┐
│10                                                                                       ⢠│
│                                                                                         ⢸│
│ 5                                                                                       ⢸│
│                                                                                         ⢸│
│ 0                                                                                       ⢸│
│   -1h                                      -30m                                       now│
└──────────────────────────────────────────────────────────────────────────────────────────┘
 1 15m   2 1h   3 3h   4 12h   q quit
```

`psys` is named apart from the zones beside it because it measures the whole platform, not
the CPU — on this hardware it reads roughly double the package, and summing it with
`core`/`uncore`/`dram` would be meaningless.

For the current boot only:

```sh
sudo chmod a+r /sys/class/powercap/intel-rapl:*/energy_uj
```

To make it stick, a udev rule granting a group you belong to:

```sh
echo 'SUBSYSTEM=="powercap", ACTION=="add", RUN+="/bin/chmod g+r /sys%p/energy_uj", RUN+="/bin/chgrp power /sys%p/energy_uj"' \
  | sudo tee /etc/udev/rules.d/99-powercap.rules
```

Either way you are re-opening the side channel to every local process that can read those
files, so weigh it against what else runs on the machine. `--rapl on` refuses to start and
tells you why, if you would rather find out than wonder.

A capability on the binary (`setcap cap_dac_read_search+ep`) is the other route often
suggested, and it is worth knowing what it costs: that capability bypasses read permission
checks on *every* file on the system, not just these. Loosening the two counters is the
narrower blast radius.

## Known limitations

- **Run in anger on exactly one machine** (Intel Panther Lake, Arch, `BAT1`, charge-reporting
  firmware). The `power_now` branch is unit-tested but has never executed against real
  hardware. Reports from other vendors welcome.
- Machines with two packs chart one of them, not the sum.
- A sharp step between widely-spaced samples fills the earlier sample's column too tall,
  because band edges are pinned to sample positions rather than interpolated. Invisible at
  1 s live cadence; visible with 30 s backfill in the 15 m range.

## Licence

MIT
