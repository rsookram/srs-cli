# srs-cli

[Spaced repetition](https://en.wikipedia.org/wiki/Spaced_repetition) on the
command line.

## Installation

Currently, pre-compiled binaries of srs-cli aren't being distributed. You can
install it with
[Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) by
running

```
cargo install --git https://github.com/rsookram/srs-cli
```

## Usage

```
USAGE:
    srs-cli [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -p, --path <PATH>    The path of the database file [default: srs.db]

SUBCOMMANDS:
    add            Create a new card
    cards          List all cards
    delete         Delete a card
    edit           Edit the contents of a card
    review         Review cards that are scheduled for review
    stats          View statistics of reviews
```

## Building

srs-cli can be built from source by cloning this repository and using Cargo.

```
git clone https://github.com/rsookram/srs-cli
cd srs-cli
cargo build --release
```

License
-------

    Copyright 2022 Rashad Sookram

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
