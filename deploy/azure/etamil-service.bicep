// An eTamil HTTP service on Azure: your program, built into a container image on top of
// the official eTamil image, run on Azure Container Apps with HTTPS ingress.
// https://github.com/Maruff/eTamil_lang
//
// What this creates: a Log Analytics workspace, a Container Apps environment and one
// container app with public HTTPS ingress. Nothing else in your subscription is touched.
//
// What it does not do: build or push your image, create a Key Vault, a managed identity or a
// container registry (see deploy/azure/README.md), or set up a custom domain. The service
// is reachable at the address Azure gives it, under azurecontainerapps.io.

targetScope = 'resourceGroup'

@description('A name for the service, used to name what is created. Lowercase letters, digits and hyphens; it must start with a letter. Container Apps allows 32 characters in all, and 8 are used by suffixes.')
@minLength(2)
@maxLength(24)
param name string = 'etamil'

@description('Your container image, built FROM ghcr.io/maruff/etamil with your program copied in, for example myregistry.azurecr.io/my-service:1.')
param image string

@description('The .qmz file to serve, relative to the image working directory (/work).')
param programPath string = 'app.qmz'

@description('The port the server listens on inside the container.')
@minValue(1)
@maxValue(65535)
param containerPort int = 8080

@description('A path your program answers with a 200. A replica that fails it is taken out of service and restarted, so route it in your program.')
param healthCheckPath string = '/'

@description('Size of each replica: vCPU, with the memory Container Apps pairs with it (0.25 vCPU / 0.5 GiB, 0.5 / 1, 1 / 2, 2 / 4, 4 / 8).')
@allowed([
  '0.25'
  '0.5'
  '1'
  '2'
  '4'
])
param size string = '0.25'

@description('The fewest replicas to keep running. 0 lets the service scale to nothing when idle, at the cost of a slow first request.')
@minValue(0)
param minReplicas int = 1

@description('The most replicas to run. Replicas are added when concurrent requests per replica pass 50.')
@minValue(1)
param maxReplicas int = 1

@description('Resource ID of a user-assigned managed identity. Needed to read a Key Vault secret (jwtSecretKeyVaultUrl) or to pull from Azure Container Registry without a password. Give it the Key Vault Secrets User role on the vault and AcrPull on the registry.')
param identityResourceId string = ''

@description('Key Vault secret URL (https://<vault>.vault.azure.net/secrets/<name>) whose value is the JWT signing secret. It is given to the program as ETAMIL_JWT_SECRET, which keeps it out of the template and the app definition. Requires identityResourceId.')
param jwtSecretKeyVaultUrl string = ''

@description('The registry the image comes from, for example myregistry.azurecr.io. Leave empty for a public image. With identityResourceId and no registryUsername, the managed identity pulls from it (Azure Container Registry).')
param registryServer string = ''

@description('Only for a private registry that is not pulled with the managed identity.')
param registryUsername string = ''

@description('Only with registryUsername. Stored as a Container Apps secret, never in the template.')
@secure()
param registryPassword string = ''

@description('How long to keep the service logs.')
@minValue(30)
@maxValue(730)
param logRetentionDays int = 30

@description('Where to create everything. Defaults to the resource group location.')
param location string = resourceGroup().location

var hasIdentity = identityResourceId != ''
var hasJwtSecret = jwtSecretKeyVaultUrl != ''
var hasRegistry = registryServer != ''
var usesPassword = hasRegistry && registryUsername != ''

var sizes = {
  '0.25': { cpu: json('0.25'), memory: '0.5Gi' }
  '0.5': { cpu: json('0.5'), memory: '1Gi' }
  '1': { cpu: json('1'), memory: '2Gi' }
  '2': { cpu: json('2'), memory: '4Gi' }
  '4': { cpu: json('4'), memory: '8Gi' }
}

var secrets = concat(
  hasJwtSecret
    ? [
        {
          name: 'jwt-secret'
          keyVaultUrl: jwtSecretKeyVaultUrl
          identity: identityResourceId
        }
      ]
    : [],
  usesPassword
    ? [
        {
          name: 'registry-password'
          value: registryPassword
        }
      ]
    : []
)

var registries = !hasRegistry
  ? []
  : [
      usesPassword
        ? {
            server: registryServer
            username: registryUsername
            passwordSecretRef: 'registry-password'
          }
        : {
            server: registryServer
            identity: identityResourceId
          }
    ]

var probe = {
  httpGet: {
    path: healthCheckPath
    port: containerPort
  }
  initialDelaySeconds: 5
  periodSeconds: 15
  failureThreshold: 3
}

resource workspace 'Microsoft.OperationalInsights/workspaces@2023-09-01' = {
  name: '${name}-logs'
  location: location
  properties: {
    sku: {
      name: 'PerGB2018'
    }
    retentionInDays: logRetentionDays
  }
}

resource environment 'Microsoft.App/managedEnvironments@2024-03-01' = {
  name: '${name}-env'
  location: location
  properties: {
    appLogsConfiguration: {
      destination: 'log-analytics'
      logAnalyticsConfiguration: {
        customerId: workspace.properties.customerId
        sharedKey: workspace.listKeys().primarySharedKey
      }
    }
  }
}

resource app 'Microsoft.App/containerApps@2024-03-01' = {
  name: name
  location: location
  identity: hasIdentity
    ? {
        type: 'UserAssigned'
        userAssignedIdentities: {
          '${identityResourceId}': {}
        }
      }
    : {
        type: 'None'
      }
  properties: {
    environmentId: environment.id
    configuration: {
      // A new revision replaces the old one only once it is healthy, so a deploy does not
      // drop the service.
      activeRevisionsMode: 'Single'
      ingress: {
        external: true
        targetPort: containerPort
        transport: 'auto'
        // Plain HTTP is redirected to HTTPS, with the certificate Azure provides.
        allowInsecure: false
      }
      registries: registries
      secrets: secrets
    }
    template: {
      containers: [
        {
          name: 'etamil'
          image: image
          resources: sizes[size]
          // The image's entrypoint is `etamil`, so these are its arguments. The server
          // listens on loopback unless told otherwise, which the ingress could not reach.
          args: [
            '--server'
            '--host'
            '0.0.0.0'
            '--port'
            string(containerPort)
            programPath
          ]
          env: hasJwtSecret
            ? [
                {
                  name: 'ETAMIL_JWT_SECRET'
                  secretRef: 'jwt-secret'
                }
              ]
            : []
          probes: [
            union({ type: 'Liveness' }, probe)
            union({ type: 'Readiness' }, probe)
          ]
        }
      ]
      scale: {
        minReplicas: minReplicas
        maxReplicas: maxReplicas
        rules: [
          {
            name: 'http'
            http: {
              metadata: {
                concurrentRequests: '50'
              }
            }
          }
        ]
      }
    }
  }
}

@description('Where the service answers.')
output url string = 'https://${app.properties.configuration.ingress.fqdn}'

@description('The Log Analytics workspace holding its logs.')
output logWorkspaceName string = workspace.name

@description('The container app, for `az containerapp logs show`.')
output appName string = app.name
