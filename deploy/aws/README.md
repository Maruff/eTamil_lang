# Deploying an eTamil service on AWS

`etamil-service.yaml` is a CloudFormation template that runs an eTamil HTTP program on
**ECS Fargate** behind an **Application Load Balancer**: no servers to look after, a new
task started before the old one stops on a deploy, and logs in CloudWatch.

You give it a container image with your program in it. The steps are: build the image,
push it, create the stack.

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

Use a version tag, not `latest`, so a rebuild does not change the compiler under you.
The image is published for `amd64` and `arm64`; build for the architecture you will run,
and set the template's `Architecture` to match (`X86_64` or `ARM64`).

## 2. Push it to Amazon ECR

An image in ECR in the same account needs no credentials in the template.

```bash
aws ecr create-repository --repository-name my-service
aws ecr get-login-password | docker login --username AWS --password-stdin <account>.dkr.ecr.<region>.amazonaws.com
docker tag my-service <account>.dkr.ecr.<region>.amazonaws.com/my-service:1
docker push <account>.dkr.ecr.<region>.amazonaws.com/my-service:1
```

## 3. Create the stack

You need a VPC and **two or more public subnets in different Availability Zones** (the
default VPC has them).

```bash
aws cloudformation deploy \
  --template-file deploy/aws/etamil-service.yaml \
  --stack-name my-service \
  --capabilities CAPABILITY_IAM \
  --parameter-overrides \
      Image=<account>.dkr.ecr.<region>.amazonaws.com/my-service:1 \
      VpcId=vpc-0123456789abcdef0 \
      SubnetIds=subnet-0aaa,subnet-0bbb \
      HealthCheckPath=/health
```

The stack's `Url` output is where it answers. Without a certificate that is plain HTTP, which is
for trying it out.

### Parameters

| Parameter | Default | What it is |
|---|---|---|
| `Image` | | Your image (required). |
| `ProgramPath` | `app.qmz` | The file to serve, relative to `/work`. |
| `ContainerPort` | `8080` | The port the server listens on in the container. |
| `HealthCheckPath` | `/` | A path your program answers with 200. A task that fails it is replaced, so **route it**. |
| `Architecture` | `X86_64` | `X86_64` or `ARM64`, matching your image. |
| `Cpu`, `Memory` | `256`, `512` | Fargate size. Fargate accepts only certain pairs. |
| `DesiredCount` | `1` | Tasks to run. Use 2 or more to stay up through a deploy or a failed task. |
| `VpcId`, `SubnetIds` | | Where to run (required). |
| `CertificateArn` | empty | An ACM certificate. If given, HTTPS on 443 and HTTP redirects to it. |
| `JwtSecretArn` | empty | A Secrets Manager secret holding the JWT signing secret, given to the program as `ETAMIL_JWT_SECRET`. |
| `RepositoryCredentialsSecretArn` | empty | Only for a private registry that is not ECR. |
| `LogRetentionDays` | `30` | How long logs are kept. |

### HTTPS and secrets

- **HTTPS:** request a certificate in AWS Certificate Manager for your domain, pass its ARN as
  `CertificateArn`, then point your domain's DNS at the load balancer (a CNAME, or an alias
  record in Route 53). The template does not manage DNS.
- **Secrets:** never put a secret in a parameter or in the image. Create it in Secrets Manager
  and pass its ARN. The task's execution role is allowed to read only the secrets you name.

## What it does and does not do

It does: run N tasks, health-check them, replace a task that fails, roll back a deploy that
cannot start (the deployment circuit breaker), keep logs, and let only the load balancer
reach the tasks.

It does **not**:

- **Scale automatically.** `DesiredCount` is fixed. Add an Application Auto Scaling target if
  you need it.
- **Use private subnets.** The tasks get public addresses so they can pull the image without
  a NAT gateway. They are reachable only from the load balancer, but a locked-down design
  would use private subnets, a NAT gateway or VPC endpoints.
- **Add a web application firewall, a domain name, a database or a cache.** Reach a database
  from your program as you would anywhere else, with the network and secret arrangements
  that needs.
- **Run in more than one region.**

Running it costs money (Fargate tasks, the load balancer and logs are billed for as long as
the stack exists). Delete it when you are done:

```bash
aws cloudformation delete-stack --stack-name my-service
```

## How it was checked

The template is linted with `cfn-lint` in CI (`.github/workflows/ci.yml`, job `cloudformation`).
**It has not been deployed to an AWS account**, so the first `deploy` may find something a linter
cannot: a quota, a Fargate CPU and memory pair that is not allowed, or a subnet that is not
public. If the stack fails to create, the CloudFormation events say which resource and why.
