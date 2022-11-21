[![Build Status](https://travis-ci.org/andrewdavidmackenzie/flow.svg?branch=master)](https://travis-ci.org/andrewdavidmackenzie/flow)
[![codecov](https://codecov.io/gh/andrewdavidmackenzie/flow/branch/master/graph/badge.svg)](https://codecov.io/gh/andrewdavidmackenzie/flow)
[![Generic badge](https://img.shields.io/badge/macos-supported-Green.svg)](https://shields.io/)
[![Generic badge](https://img.shields.io/badge/linux-supported-Green.svg)](https://shields.io/)
[![Generic badge](https://img.shields.io/badge/Rust-stable-Green.svg)](https://shields.io/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# Welcome!
Welcome to `flow`, for defining, compiling and running parallel, 
[dataflow programs](https://en.wikipedia.org/wiki/Dataflow_programming) like this one:

![First flow](first.svg)
Flow consists of a library and a binary (`flowclib` and `flowc`) for compiling flows, a library and two binaries
(`flowrlib`, `flowr` and `flowrex`) for running flows.
 
In this project I work on the programming "semantics" as I implement some sample programs.
It's a journey of discovery of writing something like this (for me), learning 
rust in the process and learning how such a paradigm could work.
 
Learn more about what it is and why I created it by reading the [Inspirations section](docs/introduction/inspirations.md)
of the book, or dive right in and see if you can understand [Your First Flow](docs/first_flow/first_flow.md) with zero previous 
knowledge and just your programmer's intuition.
 
This README.md forms part of the ['flow' Guide](http://andrewdavidmackenzie.github.io/flow/) published using gh-pages.

If you want to encourage me then you can ["patreonize me"](https://www.patreon.com/andrewmackenzie) :-)


## Building `flow`
For more details on how to build flow locally and contribute to it, please see [building flow]
(docs/developing/building.md)

## Running your first 'flow'
Run the 'fibonacci' sample flow using:

```cargo run -p flowc -- samples/fibonacci```

You should get a fibonacci series of numbers output to the terminal.

The [first flow](docs/first_flow/first_flow.md) section of the guide walks you through it.

## Sister Projects

## Are we GUI yet?
As data-flow programming fits very well with a visualization of
processes operating on data flowing from other processes, a visual programming method
would be great. But I've only touched the surface of an IDE, disappointed by the current state
of cross-platform UI programming. That's potentially a great (but massive) project
in itself.

### IDE exploration
I have been exploring how best to build a cross-platform IDE for flow that can be used to visually 
build/inspect/debug flows. The main "problem" is choosing the best cross-platform, rust-friendly GUI toolkit
to use. I have tried Iced and other emerging rust-native GUI toolkits, but they are not ready yet.
  * [flowide-gtk](https://github.com/andrewdavidmackenzie/flowide-gtk) - First attempt at creating a flow IDE 
    with a GUI, all written in rust. It is not visual yet,
  just text based but can load and compile and then run a flow. To get this to work I also spun-off: 
      * [gtk-rs-state](https://github.com/andrewdavidmackenzie/gtk-rs-state) - a small library to help
  do a GTK UI in rust, accessing UI from background threads.
        
Currently, I'm experimenting with flutter. FLTK looks relatively easy to use, has rust
native bindings for the UI, but it doesn't look so modern and is not so well known.

Once I get a solution I like, I may well bring it back into this repo, although build-times for so many crates
may become a problem again if this repo includes too much.