# Kubernetes Integration

QuicPulse provides native Kubernetes support with a special `k8s://` URL scheme for seamless access to services in Kubernetes clusters.

## Overview

The `k8s://` URL scheme allows you to make HTTP requests to Kubernetes services without manually setting up port-forwarding. QuicPulse automatically:

1. Parses the `k8s://` URL
2. Establishes a port-forward to the service
3. Sends the HTTP request through the tunnel
4. Returns the response

## URL Format

```
k8s://service.namespace[:port][/path][?query]
```

| Component | Required | Default | Description |
|-----------|----------|---------|-------------|
| `service` | Yes | - | Kubernetes service name |
| `namespace` | Yes | - | Kubernetes namespace |
| `port` | No | 80 | Service port |
| `path` | No | `/` | HTTP path |
| `query` | No | - | Query string |

## Prerequisites

- `kubectl` installed and in PATH
- Valid kubeconfig (typically `~/.kube/config`)
- Access permissions to the target namespace and service

## Basic Usage

### Simple Request

```bash
# Request to service in default namespace
quicpulse k8s://api-server.default

# Request to service in custom namespace
quicpulse k8s://grafana.monitoring
```

### With Port

```bash
# Service on port 8080
quicpulse k8s://api-server.default:8080

# Grafana on port 3000
quicpulse k8s://grafana.monitoring:3000
```

### With Path and Query

```bash
# API endpoint with path
quicpulse k8s://api-server.default:8080/api/v1/users

# With query parameters
quicpulse k8s://api-server.default:8080/search?q=test&limit=10

# Health check endpoint
quicpulse k8s://my-service.production/health
```

## Common Use Cases

### Health Checks

```bash
# Check service health
quicpulse k8s://my-service.default/health

# With assertions for CI/CD
quicpulse --assert-status 200 --assert-time "<500ms" k8s://my-service.default/health
```

### API Testing

```bash
# GET request
quicpulse k8s://api.default:8080/api/v1/users

# POST with data
quicpulse POST k8s://api.default:8080/api/v1/users name=John email=john@example.com

# With authentication
quicpulse -A bearer -a "$TOKEN" k8s://api.default:8080/api/v1/admin
```

### Debugging Services

```bash
# Verbose output
quicpulse -v k8s://my-service.default/debug

# View all headers
quicpulse -p hb k8s://my-service.default/api
```

### Internal Services

```bash
# Access internal Prometheus
quicpulse k8s://prometheus.monitoring:9090/api/v1/query?query=up

# Access internal Elasticsearch
quicpulse k8s://elasticsearch.logging:9200/_cluster/health

# Access internal Redis (HTTP interface if available)
quicpulse k8s://redis-commander.default:8081
```

## Examples by Service Type

### Web Applications

```bash
# React/Vue/Angular frontend
quicpulse k8s://frontend.default:3000

# Backend API
quicpulse k8s://backend.default:8080/api/v1/status
```

### Databases with HTTP Interfaces

```bash
# CouchDB
quicpulse k8s://couchdb.database:5984/_all_dbs

# Elasticsearch
quicpulse k8s://elasticsearch.logging:9200

# InfluxDB
quicpulse k8s://influxdb.monitoring:8086/ping
```

### Monitoring Tools

```bash
# Prometheus API
quicpulse k8s://prometheus.monitoring:9090/api/v1/targets

# Grafana API
quicpulse k8s://grafana.monitoring:3000/api/health

# AlertManager
quicpulse k8s://alertmanager.monitoring:9093/api/v1/alerts
```

### CI/CD Tools

```bash
# ArgoCD
quicpulse k8s://argocd-server.argocd:443/api/v1/applications

# Jenkins
quicpulse k8s://jenkins.ci:8080/api/json
```

## Authentication

### With Bearer Token

```bash
# Use service account token
quicpulse -A bearer -a "$(kubectl get secret my-token -o jsonpath='{.data.token}' | base64 -d)" \
  k8s://api.default:8080/api/v1/admin
```

### With Basic Auth

```bash
# Basic auth for internal services
quicpulse -a admin:password k8s://internal-service.default/admin
```

### With Kubernetes Token

```bash
# Use current context's token
TOKEN=$(kubectl config view --raw -o jsonpath='{.users[0].user.token}')
quicpulse -A bearer -a "$TOKEN" k8s://api.default:8080/api/v1/secure
```

## Workflows with Kubernetes

### Kubernetes Service Testing

```yaml
# k8s-tests.yaml
name: Kubernetes Service Tests

variables:
  namespace: production

steps:
  - name: Health check
    request:
      url: "k8s://api-server.{{ namespace }}:8080/health"
    assertions:
      status: 200
      time: "<500ms"

  - name: API version
    request:
      url: "k8s://api-server.{{ namespace }}:8080/api/version"
    assertions:
      body:
        - ".version"

  - name: List users
    request:
      url: "k8s://api-server.{{ namespace }}:8080/api/v1/users"
    assertions:
      status: 200
      headers:
        - "Content-Type:application/json"
```

### Run with different namespaces

```bash
# Development
quicpulse --run k8s-tests.yaml --var namespace=development

# Staging
quicpulse --run k8s-tests.yaml --var namespace=staging

# Production
quicpulse --run k8s-tests.yaml --var namespace=production
```

## Validation

QuicPulse validates Kubernetes resource names according to DNS naming rules:

| Rule | Valid | Invalid |
|------|-------|---------|
| Max 63 characters | `my-service` | `very-long-service-name-that-exceeds-sixty-three-characters-limit` |
| Lowercase letters and digits | `api-v2` | `API-V2` |
| Start with letter | `api-server` | `2nd-api` |
| End with alphanumeric | `api-v2` | `api-server-` |
| Hyphens allowed (not start/end) | `my-api-server` | `-api-` |

## Error Handling

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| `kubectl not found` | kubectl not installed | Install kubectl |
| `Unable to connect` | Cluster not accessible | Check kubeconfig |
| `Service not found` | Service doesn't exist | Verify service name and namespace |
| `Permission denied` | RBAC restrictions | Check service account permissions |
| `Connection refused` | Service not ready | Check pod status |

### Debugging Connection Issues

```bash
# Verify service exists
kubectl get svc api-server -n default

# Check endpoints
kubectl get endpoints api-server -n default

# Check pod status
kubectl get pods -l app=api-server -n default

# Manual port-forward test
kubectl port-forward svc/api-server 8080:8080 -n default
```

## How It Works

1. **URL Parsing**: QuicPulse parses the `k8s://` URL to extract service, namespace, port, and path

2. **Port Selection**: QuicPulse selects an available local port for the tunnel

3. **Port-Forward**: QuicPulse runs `kubectl port-forward` in the background:
   ```bash
   kubectl port-forward svc/api-server 12345:8080 -n default
   ```

4. **Request**: The HTTP request is sent to `localhost:12345`

5. **Response**: The response is returned through the tunnel

6. **Cleanup**: The port-forward is terminated after the request

## Comparison with Manual Port-Forward

### Manual Approach

```bash
# Terminal 1: Start port-forward
kubectl port-forward svc/api-server 8080:8080 -n default

# Terminal 2: Make request
curl http://localhost:8080/api/v1/users
```

### With QuicPulse

```bash
# Single command
quicpulse k8s://api-server.default:8080/api/v1/users
```

## Performance Considerations

- First request may be slower due to port-forward setup
- Consider using sessions for multiple requests to same service
- Port-forward is automatically cleaned up after request

## Limitations

- Requires `kubectl` in PATH
- Only supports HTTP (not raw TCP)
- One request per port-forward (no connection pooling)
- Depends on kubeconfig context

## See Also

- [CLI Reference](cli-reference.md) - All command-line flags
- [Workflows](workflow.md) - Automated testing with Kubernetes
- [Assertions](assertions.md) - Testing Kubernetes services
