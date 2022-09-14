# CHIP-8 emulator

### Requirements
SFML:
- `sudo apt install libsfml-dev libcsfml-doc`
- `sudo pacman -S sfml csfml`

CHIP-8 ROMs:
[Download CHIP-8 game pack](https://www.zophar.net/pdroms/chip8/chip-8-games-pack.html)

  
 
### Compiling
`cargo build --release`


### Run
Pass the file path to ROM file as argument:

`./target/release/chip8-rust ~/<dir to CHIP-8 games>/FILENAME`
