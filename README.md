# ti

A simple terminal timer

## Installation

Install Rust :crab:

```sh
$ sudo pacman -S rustup
```

Initialize default stable

```sh
$ rustup default stable
```

Clone this repo

```sh
$ git clone https://github.com/eRgo35/ti
```

Change directory

```sh
$ cd ti
```

```sh
$ cargo aur
```

```sh
$ cd target/cargo-aur
```

Install package

```sh
$ makepkg -si
```

## Usage

```
$ ti --help
A simple terminal timer

Usage: ti [OPTIONS]

Options:
  -H, --hours <HOURS>      Hours [default: 0]
  -M, --minutes <MINUTES>  Minutes [default: 0]
  -S, --seconds <SECONDS>  Seconds [default: 0]
      --font <FONT>        Path to custom font file [default: ]
      --cache <CACHE>      Path to cache file [default: ]
  -h, --help               Print help
  -V, --version            Print version
```
