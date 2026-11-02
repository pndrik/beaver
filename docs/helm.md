# Helm chart

The Helm chart installs Beaver's webhook server on a Kubernetes cluster. Your configuration, agents and flows are Helm values, so adding an agent or a flow is a `helm upgrade`. This guide goes from an empty cluster to a running flow, then covers secrets, exposing hooks, custom tools and every value.

## Prerequisites

- A Kubernetes cluster and `kubectl` access to it.
- Helm 3.8 or newer (for OCI registries).
- An API key of an [inference provider](configuration.md#inference-providers-and-models), e.g. Anthropic.

The chart is published to `oci://ghcr.io/pndrik/charts/beaver` with each [release](https://github.com/pndrik/beaver/releases). Chart and image share the version, e.g. `v0.0.20`. Below, replace `<version>` with the release you want.

## 1. Install

Create a values file with the provider. The API key stays out of it: export it once, and pass it with every `helm install` and `helm upgrade`:

```yaml
# values.yaml
configuration:
  inference:
    providers:
      anthropic:
        enabled: true
```

```sh
export ANTHROPIC_API_KEY="<your_anthropic_api_key>"

helm install beaver oci://ghcr.io/pndrik/charts/beaver \
  --version <version> \
  --namespace beaver --create-namespace \
  -f values.yaml \
  --set-string env.INFERENCE__PROVIDERS__ANTHROPIC__APIKEY="$ANTHROPIC_API_KEY"
```

To install from a checkout of the repository instead, use `./chart` in place of the OCI URL and drop `--version`.

## 2. Check that it runs

```sh
kubectl -n beaver get pods
kubectl -n beaver logs deploy/beaver-webhook
kubectl -n beaver port-forward svc/beaver-webhook 8080:80
curl localhost:8080/healthz   # OK
```

The log should end with `Web server listening`. If the pod restarts, the log shows why, e.g. `Failed to bootstrap application: …` for a broken configuration. See [Troubleshooting](#troubleshooting).

## 3. Add an agent and a flow

Agents and flows are maps under `agents` and `flows`. The key is the file name, so it must equal `metadata.name`. The content is the same YAML as in [agents](agents.md) and [flows](flows.md):

```yaml
# values.yaml
configuration:
  inference:
    providers:
      anthropic:
        enabled: true

agents:
  summarizer:
    version: 1
    metadata:
      name: summarizer
      display_name: Sumi
      description: Summarizes text in three sentences.
    spec:
      inference:
        model: claude-sonnet-5-5
      prompt: |
        Summarize the text you get in at most three short sentences.

flows:
  summarize:
    version: 1
    metadata:
      name: summarize
    spec:
      arguments:
        text: { required: true }
      contexts:
        main: { agent: summarizer }
      triggers:
        webhook:
          name: summarize
          token: "<your_flow_token>"
          arguments:
            text: "{{ Body.text }}"
      steps:
        - name: summarize
          context: main
          type: agent
          arguments:
            prompt: "{{ Arguments.text }}"
```

Apply it:

```sh
helm upgrade beaver oci://ghcr.io/pndrik/charts/beaver \
  --version <version> --namespace beaver \
  -f values.yaml \
  --set-string env.INFERENCE__PROVIDERS__ANTHROPIC__APIKEY="$ANTHROPIC_API_KEY"
```

Avoid `--reuse-values`: it keeps old values, so agents and flows you remove from your files would stay installed.

The pod restarts with the new definitions. Trigger the flow through the port-forward from step 2:

```sh
curl -H 'Authorization: Bearer <your_flow_token>' -H 'Content-Type: application/json' \
  -d '{"text": "Beavers are large rodents that build dams..."}' localhost:8080/hooks/summarize
kubectl -n beaver logs deploy/beaver-webhook
```

The hook answers `{"success":true,"message":"Inference started"}` right away. The flow runs in the background, and its progress and errors are in the log. Set `configuration.logger.level: debug` to also see prompts and replies.

Templates like `{{ Body.text }}` in agents and flows are passed to Beaver as they are; Helm doesn't evaluate them.

## Keeping specs in their own files

With more than a few agents and flows, one values file gets long. Helm merges several `-f` files, so you can keep one file per agent or flow:

```text
beaver/
├── values.yaml              # configuration, ingress, resources
├── agents/
│   ├── summarizer.yaml      # agents: { summarizer: … }
│   └── reviewer.yaml        # agents: { reviewer: … }
└── flows/
    └── summarize.yaml       # flows: { summarize: … }
```

```yaml
# beaver/agents/reviewer.yaml
agents:
  reviewer:
    version: 1
    metadata:
      name: reviewer
      display_name: Rev
      description: Reviews the code in /repo.
    spec:
      inference:
        model: claude-sonnet-5-5
      tools:
        allow:
          - name: ^file_.*$
            scopes: [/repo]
      prompt: |
        Review the code in /repo and list the problems you find.
```

```sh
helm upgrade beaver oci://ghcr.io/pndrik/charts/beaver \
  --version <version> --namespace beaver \
  -f beaver/values.yaml $(for f in beaver/agents/*.yaml beaver/flows/*.yaml; do printf -- '-f %s ' "$f"; done) \
  --set-string env.INFERENCE__PROVIDERS__ANTHROPIC__APIKEY="$ANTHROPIC_API_KEY"
```

Every file must wrap its definition in `agents:` or `flows:` and the name. Don't use `--set-file` for definitions: it passes the file as a string, and Beaver can't load it. An agent or flow removed from all files is removed from the pod on the next upgrade.

Test new agents locally before you upgrade: `cargo run -p cli -- agents list` shows validation errors right away, see [CLI](cli.md#debugging).

## Secrets

The chart writes your configuration, agents, flows and `env` into Kubernetes Secrets. Keep API keys, tokens and SSH keys out of version control:

- Pass them as environment variables with `--set-string env.<NAME>=…`, e.g. from your CI's secret store. Every configuration key can be set this way, with `__` between levels: `INFERENCE__PROVIDERS__ZEN__APIKEY`, `TOOLS__CONFIGURATION__JS_SLACK__API_KEY`. See [configuration sources](configuration.md#sources).
- Keep them in a separate, encrypted values file, e.g. with [helm-secrets](https://github.com/jkroepke/helm-secrets) or [SOPS](https://github.com/getsops/sops), and add it with another `-f`.

Lists, like MCP servers or git repositories, can't be set through environment variables; put them in an encrypted values file. `env` values are rendered with Helm's `tpl`, so they can reference other values, but a literal `{{` in them must be escaped.

## Exposing hooks

To let GitHub, Bitbucket, Jira or another service call your flows, enable the Ingress. It only exposes `/hooks`; `/healthz` stays inside the cluster.

```yaml
# values.yaml
ingress:
  enabled: true
  host: beaver.example.com
  ingressClassName: nginx
  annotations:
    nginx.ingress.kubernetes.io/proxy-body-size: 1m
```

The chart's Ingress has no `tls` section. Terminate TLS with your Ingress controller's default certificate, a load balancer in front, or your own Ingress through `extraObjects`. Hooks then answer on `https://beaver.example.com/hooks/<name>`. Send the token as an `Authorization: Bearer` header where the caller supports it, see [security](security.md#webhooks).

## Tools, git and MCP

Everything under `configuration` is Beaver's [configuration](configuration.md), so tools are set up there. Some examples:

- Git repositories with SSH keys: [example](configuration.md#git-repositories-with-an-ssh-key).
- MCP servers: [example](configuration.md#mcp-servers).
- Configuration for JavaScript tool packages: [example](configuration.md#javascript-tools-with-per-package-configuration).

The image ships the example tool packages in `/tools`, and the chart enables the JavaScript provider on that path. To add your own packages, build an image on top of Beaver's and point the chart at it:

```dockerfile
FROM ghcr.io/pndrik/beaver:<version>
COPY tools/ /tools/
```

```yaml
# values.yaml
image:
  repository: registry.example.com/beaver
  tag: "<version>-tools"
```

See [JavaScript tools](tools.md#javascript-tools) for the package format.

## Storage and scaling

Agents work in `file_system.disk.root`, which the chart sets to `/sandbox`, an `emptyDir`. Each pod has its own, and its content is lost when the pod restarts. A flow runs in the pod that received its hook, so flows that clone repositories into a chroot work with any number of replicas. Don't rely on files surviving between flow runs.

The HorizontalPodAutoscaler scales between `webhook.hpa.minReplicas` and `maxReplicas` on CPU. Agents mostly wait for the inference provider, so CPU stays low; raise `minReplicas` if you expect many parallel flows.

## Values

| Key | Default | Description |
|---|---|---|
| `nameOverride` | `beaver` | Name used in resource names, see [Resource names](#resource-names). |
| `image.repository` | `ghcr.io/pndrik/beaver` | Image to deploy. |
| `image.tag` | the chart's `appVersion` | Image tag. |
| `image.pullPolicy` | `IfNotPresent` | Image pull policy. |
| `env` | `{ CONFIGURATION__PATH: /configuration.yaml }` | Environment variables of the container. Values are templated. Keep `CONFIGURATION__PATH`, Beaver reads the configuration from there. |
| `configuration` | see below | Beaver's `configuration.yaml`, mounted at `/configuration.yaml`. Templated. |
| `agents` | `{}` | Map of agent name → [agent definition](agents.md). Each entry becomes `/agents/<name>.yaml`. |
| `flows` | `{}` | Map of flow name → [flow definition](flows.md). Each entry becomes `/flows/<name>.yaml`. |
| `ingress.enabled` | `false` | Expose `/hooks` through an Ingress. |
| `ingress.host` | `beaver.example.com` | Host of the Ingress rule. |
| `ingress.ingressClassName` | — | Ingress class. |
| `ingress.labels`, `ingress.annotations` | `{}` | Extra labels and annotations of the Ingress. |
| `webhook.service.type` | `ClusterIP` | Service type. |
| `webhook.service.port` | `80` | Service port, forwarded to the container's `8080`. |
| `webhook.service.labels`, `webhook.service.annotations` | `{}` | Extra labels and annotations of the Service. |
| `webhook.hpa.minReplicas`, `webhook.hpa.maxReplicas` | `1`, `3` | Replica range of the autoscaler. The autoscaler is always created. |
| `webhook.hpa.metrics` | CPU at 80 % | Autoscaler metrics. |
| `webhook.resources` | requests `100m`/`256Mi`, limits `1`/`256Mi` | Container resources. |
| `webhook.securityContext` | user and group `1000` | Pod security context. |
| `extraObjects` | `[]` | Additional manifests, as strings or objects. Templated. |

The default `configuration` sets `file_system.disk.root: /sandbox`, `agents.path: /agents`, `flows.path: /flows` and enables the JavaScript provider with `path: /tools`. Helm merges your `configuration` into it, so you only set what differs. Don't change these paths: the chart mounts the volumes there.

[`chart/values.example.yaml`](../chart/values.example.yaml) is a complete example: Anthropic, an Atlassian MCP server, configuration for the Bitbucket and Slack tools and a git repository, a `reviewer` agent, and a `pr_review_flow` flow that checks out a pull request before reviewing it.

### Resource names

Resource names start with `<release>-beaver`, or with `<release>` if the release name already contains `beaver`. With `helm install beaver …` the Deployment and Service are `beaver-webhook`, and the Secrets are:

| Secret | Content |
|---|---|
| `beaver-configuration` | `configuration.yaml` |
| `beaver-agent-definitions` | One `<name>.yaml` per entry of `agents` |
| `beaver-flow-definitions` | One `<name>.yaml` per entry of `flows` |
| `beaver-environment` | The `env` values |

Changing any of them restarts the pod. The definition and environment Secrets were renamed in 0.0.22; the old `-agents`, `-flows` and `-env` Secrets are deleted on upgrade.

## Upgrade and uninstall

```sh
helm upgrade beaver oci://ghcr.io/pndrik/charts/beaver --version <version> --namespace beaver -f values.yaml \
  --set-string env.INFERENCE__PROVIDERS__ANTHROPIC__APIKEY="$ANTHROPIC_API_KEY"
helm uninstall beaver --namespace beaver
```

Read the [release notes](https://github.com/pndrik/beaver/releases) before you upgrade; Beaver is experimental, and definitions can change between versions.

## Troubleshooting

| Symptom | Cause |
|---|---|
| Pod restarts, log shows `Failed to bootstrap application: …` | Invalid configuration, e.g. an unknown provider or a missing tool path. |
| Hook answers `401 Invalid token` | Wrong token, no flow with this `triggers.webhook.name`, or the flow is invalid: look for `Skipping flow` in the log. |
| Hook answers `422 Invalid arguments` | The request doesn't match the flow's arguments; the log line `Hook rejected` names the problem. |
| Hook answers `200`, but nothing happens | The flow failed in the background; look for `Hook failed` in the log. |
| `No inference provider found for model` | The provider isn't enabled, its API key is missing, or it doesn't serve the model. |
| `Agent '<name>' not found.` | The key under `agents` doesn't match the name the flow uses. |
