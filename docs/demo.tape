# Recorded with https://github.com/charmbracelet/vhs — `vhs docs/demo.tape`
# Run from the repo root after `cargo build --release`.

Output docs/demo.gif

Set Shell "bash"
Set FontSize 13
Set Width 1060
Set Height 560
Set Padding 20
Set Theme "Catppuccin Mocha"

# vhs does not advertise COLORTERM, so ask for 24-bit explicitly rather than letting
# auto-detection fall back to named colours and flatten the gradient.
Hide
Type "./target/release/wattmeter --color truecolor --range 1h"
Enter
Sleep 4s
Show

Sleep 3s

# The range keys, which a static screenshot cannot show.
Type "3"
Sleep 3s
Type "4"
Sleep 3s
Type "1"
Sleep 3s
Type "2"
Sleep 3s

Hide
Type "q"
