# gmk104-linux

CLI tool for uploading images and GIFs to the GMK104 keyboard LCD.

## Install

```bash
# macOS
brew install hidapi pkg-config libusb

# Linux (Debian/Ubuntu)
sudo apt install libhidapi-dev pkg-config

# Build
cargo build --release
```

### Linux permissions

Copy the udev rules so you don't need sudo:

```bash
sudo cp udev/99-gmk104.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Add yourself to the `plugdev` group and re-login:
```bash
sudo usermod -a -G plugdev $USER
```

## Usage

### Upload an image or GIF

```bash
gmk104ctl upload my-image.png
gmk104ctl -vvv upload animation.gif # extra verbose output
gmk104ctl -v upload my-image.png   # verbose output
```

### Switch display

```bash
gmk104ctl show 0      # slot 0
gmk104ctl show 1      # slot 1
gmk104ctl show time   # clock display
```

### Sync time

```bash
gmk104ctl sync-time
```

### Other commands

```bash
gmk104ctl list                    # find devices
gmk104ctl probe --handshake -v    # test protocol
gmk104ctl init                    # reset LCD
```

## Options

Global: `--vid`, `--pid`, `--path`, `-v/-vv/-vvv`

Upload: `--no-resize`, `--color "255,0,0"` (solid color test)

## License

MIT
