Anabeeb
-------

Anabeeb is a [MIDI](https://en.wikipedia.org/wiki/MIDI) control program designed to simulate a pipe organ.
Written in the [Rust](https://en.wikipedia.org/wiki/Rust_(programming_language)) programming language,
it delivers optimal performance with minimal latency and runs on all major operating systems.

At startup, the application loads a configuration file (the organ disposition) and initializes all connected hardware.
Once running, the organ can be controlled through various inputs—including MIDI devices, the console, a REST API, and
UI interactions. These inputs are translated into MIDI events that drive sound production.

Anabeeb supports multiple sound engines, with built-in integration for [FluidSynth](https://www.fluidsynth.org/) as the default option.

The user interface is built with HTML, CSS, and a lightweight JavaScript library, leveraging the high-performance rendering
engines of modern browsers.
This approach keeps the core organ processing fast and efficient while allowing the UI to run independently — even on a
different device within the same network.
An experimental HTML user interface is provided, offering limited editing capabilities via contextmenu.

Organ Disposition
-----------------

An organ disposition defines a network of interconnected elements that coordinate through events:

  - Keyboards receive MIDI input and translate it into key press and release events.

  - Consoles modify the state of other elements.

  - Combinations and memories capture and recall element states.

  - Sound elements are responsible for producing sound.

At the heart of this system is the Coupler, which routes key presses according to its active state, enabling complex
interactions between manuals, pedals, and stops.

A disposition is stored as a JSON text file that can be edited with any text editor. Each release contains an example disposition
that can be used as a basis for building your own virtual organ.

Usage
-----

Anabeeb is a console application. By default, it loads the configuration from `disposition.json` in the current folder.
If no arguments are provided, this file will be used automatically.

To see available command-line options, run:

> anabeeb --help

When reusing an existing disposition, you may need to adapt it to your specific hardware setup.
To do so, start Anabeeb with the `--setup` argument:

> anabeeb --setup disposition.json

This launches an interactive setup process where you can select MIDI devices and adjust basic configurations.
Your changes are saved in a separate file `disposition.override.json`, keeping your customizations independent from the
original disposition so it can be updated or replaced without losing your settings.

How to build on windows
-----------------------

- install Git
  - download setup from https://git-scm.com/downloads/win
- install Rust
  - download rustup-init.exe (64-BIT) from https://rustup.rs/
  - choose 1) to install Visual Studio
  - wait for Visual Studio to be installed
  - press enter to install Rust with defaults
- install virtual keyboard (optional)
  - download from https://sourceforge.net/projects/vmpk/files/
- install virtual midi port (optional)
  - download from https://www.tobias-erichsen.de/software/loopmidi.html
- get sources
  > git clone https://github.com/svenmeier/anabeeb  
    cd anabeeb  
- install fluidsynth dependency
  > download-fluidsynth.bat 
- build
  > cargo build
- run
  > cargo run 

how to build on linux
---------------------

- install tools
  > sudo apt update  
    sudo apt install git  
    sudo apt install snapd  
    sudo reboot  
    sudo snap install core
- install fluidsynth dependency
  > sudo apt install libfluidsynth3 libfluidsynth-dev
- install rust
  > snap install --classic rustup  
    rustup install stable  
    cargo install cargo-deb  
- install virtual keyboard
  > sudo apt install vmpk  	
- get sources
  > git clone https://github.com/svenmeier/anabeeb.git  
    cd anabeeb
- build 
  > cargo build  
- run
  > cargo run
- build deb package
  > cargo deb
- install deb
  > sudo apt -f install ./target/debian/*.deb

References
----------

The program name Anabeeb comes from the Arabic word أنابيب for pipes.

Anabeeb is designed as the successor to the jOrgan virtual organ and strives for near-complete compatibility with its predecessor’s features — most notably excluding MPL support.\
Unlike jOrgan, which relies on a graphical editor, Anabeeb encourages creators to take advantage of the flexibility of JSON.
To support this, it includes schema definitions for all file formats, enabling a faster and precise editing workflow
with any suitable JSON editor.

Images used from www.freepik.com