# Panda Pet Assets

Place panda GIF animations here. Each file should be:
- Resolution: 240x135 pixels (LCD native)
- Format: GIF (animated or static)
- Named exactly as listed below

## Required Files

| File | Mood | Description |
|------|------|-------------|
| happy.gif | Happy | No errors - panda bobbing head, smiling |
| worried.gif | Worried | Few errors - sweat drop, looking nervous |
| sad.gif | Sad | Several errors - droopy ears, tear |
| angry.gif | Angry | Many errors - red face, steam |
| excited.gif | Excited | Build success - jumping, sparkles |
| bored.gif | Bored | Idle 5 min - yawning |
| sleeping.gif | Sleeping | Idle 15 min - lying down, Zzz |
| eating.gif | Eating | Break time - chomping bamboo |

## Quick Test

Run the placeholder generator to create colored test images:
```bash
cargo run -p pet-daemon --bin generate-placeholders
```
