# chmi

`chmi` is a command-line tool for changing monitor inputs on Windows.

## Usage

`chmi` is interactive. It prompts you for a monitor and an input. Here's an example run:

```
$ chmi
  1 LG HDR 4K
  2 U32J59x
  3 VG259
==> Monitor (1/2/3): 3
  1 HDMI 1 (*)
  2 HDMI 2
  3 DisplayPort 1
==> Input (1/2/3): 3
```

See `chmi --help` for available options.

## Why

I have a monitor that's shared between a Windows and Linux machine. I got
tired of reaching around my monitor to switch its input. `chmi` uses the Win32
Monitor Configuration API to change monitor inputs programatically. I'm sure
there are existing tools to do the same thing, but it seemed more fun to write
my own.

## Resources

- Monitor Configuration API: https://learn.microsoft.com/en-us/windows/win32/api/_monitor/