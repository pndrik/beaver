# Deployment

Beaver runs as a container with the webhook server. The [Helm chart](helm.md) deploys it to Kubernetes with your configuration, agents and flows.

## Docker image

Each version tag (`v*`) publishes a multi-arch image (amd64, arm64) to `ghcr.io/pndrik/beaver:<version>`, e.g. `ghcr.io/pndrik/beaver:v0.0.20`. See the [releases](https://github.com/pndrik/beaver/releases). The image is built from `scratch` and contains:

| Path | Content |
|---|---|
| `/webhook` | The webhook server, the entrypoint. It listens on port `8080`. |
| `/cli` | The [CLI](cli.md). |
| `/tools` | The example JavaScript tool packages from [`examples/tools/`](../examples/tools). |
| `/tmp` | Empty directory, the default `file_system.disk.root`. |

```sh
docker run -p 8080:8080 \
  -e CONFIGURATION__PATH=/configuration.yaml \
  -e INFERENCE__PROVIDERS__ANTHROPIC__APIKEY="<your_anthropic_api_key>" \
  -v "$PWD/configuration.yaml:/configuration.yaml:ro" \
  -v "$PWD/agents:/agents:ro" \
  -v "$PWD/flows:/flows:ro" \
  ghcr.io/pndrik/beaver:<version>
```

With `agents.path: /agents` and `flows.path: /flows` in `configuration.yaml`. See [configuration](configuration.md).

## Helm chart

The chart in [`chart/`](../chart) deploys the webhook server to Kubernetes, with your configuration, agents and flows as Helm values:

```sh
helm install beaver oci://ghcr.io/pndrik/charts/beaver --version <version> -f values.yaml
```

See [Helm chart](helm.md) for a step-by-step installation, adding agents and flows, secrets, exposing hooks and every value.

## Building from source

Beaver is a Rust workspace: `domains` holds the core logic, `adapters` implement inference, tools, configuration and storage, `app` wires them together, and `platforms/cli` and `platforms/webhook` are the two binaries.

```sh
cargo build --release
./target/release/webhook
```

Or build the image with the [`Dockerfile`](../Dockerfile): `docker build -t beaver .`.
