# Santa Tracker 🎅

A festive Rust application that tracks Santa's journey around the world on Christmas Eve with beautiful terminal RGB effects, animated snowflakes, Christmas trees with ornaments, and real-time position updates.

## Features

- 🎄 **RGB Terminal Effects**: Colorful animations with rainbow effects
- ❄️ **Animated Snowflakes**: Falling snow with different characters and speeds
- 🌲 **Christmas Trees**: Decorated trees with colorful ornaments and star toppers
- 🎅 **Real-time Tracking**: Santa's position updates based on timezone progression
- 🎁 **Present Counter**: Track gifts delivered across the globe
- 🛷 **Sleigh Animation**: Animated sleigh moving across the screen

## Quick Start

### Local Development

```bash
# Build and run
cargo run --release
```

Press `q` or `ESC` to quit.

### Docker

```bash
# Build the image
docker build -t santa-tracker:latest .

# Run the container (interactive terminal)
docker run -it --rm santa-tracker:latest
```

### Kubernetes Deployment

```bash
# Build and load image to your cluster
docker build -t santa-tracker:latest .

# For kind clusters
kind load docker-image santa-tracker:latest

# For k3s/other clusters, push to your registry first
# docker tag santa-tracker:latest your-registry/santa-tracker:latest
# docker push your-registry/santa-registry:latest

# Deploy to Kubernetes
kubectl apply -f k8s/deployment.yaml

# View logs (interactive terminal output)
kubectl logs -f -n santa-tracker deployment/santa-tracker

# Or attach to the pod for interactive mode
kubectl attach -it -n santa-tracker deployment/santa-tracker
```

## How It Works

Santa's journey begins at the North Pole on December 24th at 6:00 PM local time and follows timezone progression around the world:

1. **Tokyo, Japan** 🇯🇵
2. **Sydney, Australia** 🇦🇺
3. **Beijing, China** 🇨🇳
4. **Mumbai, India** 🇮🇳
5. **Dubai, UAE** 🇦🇪
6. **Moscow, Russia** 🇷🇺
7. **Berlin, Germany** 🇩🇪
8. **London, UK** 🇬🇧
9. **New York, USA** 🇺🇸
10. **Los Angeles, USA** 🇺🇸
11. **Honolulu, Hawaii** 🇺🇸

## Technical Details

- **Language**: Rust 2021 Edition
- **Terminal UI**: Crossterm for cross-platform terminal manipulation
- **Async Runtime**: Tokio for smooth animations
- **Color Support**: RGB true color terminal support
- **Container**: Multi-stage Docker build for ~30MB final image

## Dependencies

- `crossterm` - Terminal manipulation and colors
- `tokio` - Async runtime
- `rand` - Random number generation for effects
- `chrono` - Date and time calculations
- `colored` - Terminal color support
- `serde` & `serde_json` - Configuration serialization

## Requirements

- Rust 1.75 or later
- Terminal with true color support (most modern terminals)
- Docker (for containerization)
- Kubernetes cluster (for K8s deployment)

## License

MIT

🎄 Merry Christmas! 🎅
