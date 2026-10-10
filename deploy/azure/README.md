# Deploying an eTamil service on Azure

`etamil-service.bicep` is a Bicep template that runs an eTamil HTTP program on
**Azure Container Apps**: HTTPS with a certificate Azure provides, a new revision started
before the old one stops on a deploy, and logs in Log Analytics. There are no servers or
Kubernetes to look after.

You give it a container image with your program in it. The steps are: build the image, push
it, deploy the template.

## 1. Build your image

The official image has the compiler and the standard library and runs `etamil`. Put your
program on top of it:

```dockerfile
FROM ghcr.io/maruff/etamil:1.4.2
COPY --chown=etamil app.qmz /work/app.qmz
```

```bash
docker build -t my-service .
```

Use a version tag, not `latest`, so a rebuild does not change the compiler under you. The
image is published for `amd64` and `arm64`; Container Apps runs `amd64`, so build for that.

## 2. Push it to Azure Container Registry

```bash
az group create --name my-service-rg --location centralindia
az acr create --resource-group my-service-rg --name <registry> --sku Basic
az acr login --name <registry>
docker tag my-service <registry>.azurecr.io/my-service:1
docker push <registry>.azurecr.io/my-service:1
```

A public image (for example on GitHub Container Registry) needs no registry at all: leave
`registryServer` empty.

## 3. Let the app pull from the registry

The app pulls with a **managed identity**, so there is no password to store. Create one and
give it `AcrPull` on the registry:

```bash
az identity create --resource-group my-service-rg --name my-service-id
az role assignment create \
  --assignee $(az identity show -g my-service-rg -n my-service-id --query principalId -o tsv) \
  --role AcrPull \
  --scope $(az acr show --name <registry> --query id -o tsv)
```

## 4. Deploy

```bash
az deployment group create \
  --resource-group my-service-rg \
  --template-file deploy/azure/etamil-service.bicep \
  --parameters \
      image=<registry>.azurecr.io/my-service:1 \
      registryServer=<registry>.azurecr.io \
      identityResourceId=$(az identity show -g my-service-rg -n my-service-id --query id -o tsv) \
      healthCheckPath=/health
```

The deployment's `url` output is where it answers, over HTTPS.

### Parameters

| Parameter | Default | What it is |
|---|---|---|
| `name` | `etamil` | Names what is created (`<name>`, `<name>-env`, `<name>-logs`). |
| `image` | | Your image (required). |
| `programPath` | `app.qmz` | The file to serve, relative to `/work`. |
| `containerPort` | `8080` | The port the server listens on in the container. |
| `healthCheckPath` | `/` | A path your program answers with 200. A replica that fails it is taken out of service and restarted, so **route it**. |
| `size` | `0.25` | vCPU per replica (`0.25`, `0.5`, `1`, `2`, `4`), with the memory Container Apps pairs with it. |
| `minReplicas`, `maxReplicas` | `1`, `1` | Replicas to run. Raise `maxReplicas` and replicas are added above 50 concurrent requests each. `minReplicas` 0 scales to nothing when idle, with a slow first request. |
| `identityResourceId` | empty | A user-assigned managed identity, for pulling from ACR and reading Key Vault. |
| `jwtSecretKeyVaultUrl` | empty | A Key Vault secret URL, given to the program as `ETAMIL_JWT_SECRET`. Needs `identityResourceId`. |
| `registryServer` | empty | The registry the image comes from. |
| `registryUsername`, `registryPassword` | empty | Only for a private registry that is not pulled with the identity. |
| `logRetentionDays` | `30` | How long logs are kept (30 to 730). |
| `location` | the resource group's | Where to create it. |

### Secrets

Never put a secret in a parameter or the image. Create it in **Key Vault**, give the managed
identity the `Key Vault Secrets User` role on the vault, and pass the secret's URL as
`jwtSecretKeyVaultUrl`. Container Apps reads it at start and gives it to the program as
`ETAMIL_JWT_SECRET`; the value never appears in the template or the app definition.

```bash
az role assignment create --assignee <identity principalId> \
  --role "Key Vault Secrets User" --scope <vault resource id>
```

### Logs

```bash
az containerapp logs show --resource-group my-service-rg --name etamil --follow
```

or query the Log Analytics workspace in the portal.

## What it does and does not do

It does: run replicas, health-check them, restart one that fails, keep the old revision
serving until the new one is healthy, keep logs, and serve HTTPS (HTTP is redirected to it).

It does **not**:

- **Create the registry, the identity or the Key Vault.** Their commands are above, so you
  keep control of who can read what.
- **Set up a custom domain.** The service answers at an address under `azurecontainerapps.io`.
  A custom domain and certificate are added to the container app afterwards.
- **Use a private network.** Ingress is public. A locked-down design would put the
  environment in a virtual network with private ingress and a gateway in front.
- **Add a web application firewall, a database or a cache.** Reach a database from your program
  as you would anywhere else.
- **Run in more than one region.**

Running it costs money (replicas and Log Analytics ingestion are billed while they exist). Delete
everything when you are done:

```bash
az group delete --name my-service-rg
```

## How it was checked

The template is compiled and linted with the Bicep compiler in CI (`.github/workflows/ci.yml`,
job `bicep`). **It has not been deployed to an Azure subscription**, so the first deployment may
find something a linter cannot: a quota, a region without Container Apps, a role assignment that
has not yet propagated (it can take a minute or two after creating the identity), or a property
value Azure rejects. If it fails, the deployment's error says which resource and why.
