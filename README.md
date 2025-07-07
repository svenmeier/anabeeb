how to build on windows
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
- run with one of these
  > cargo run
- build deb package
  > cargo build --release  
    cargo deb
- install deb
  > sudo apt -f install ./target/debian/*.deb