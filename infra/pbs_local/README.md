# PBS Docker Cluster

**PBS Docker Cluster** is a multi-container PBS Pro cluster designed for rapid
deployment using Docker Compose. This repository simplifies the process of
setting up a robust PBS Pro environment for development, testing, or lightweight
usage.

## 🏁 Getting Started

To get up and running with PBS Pro in Docker, make sure you have the following tools installed:

- **[Docker](https://docs.docker.com/get-docker/)**
- **[Docker Compose](https://docs.docker.com/compose/install/)**

## 📦 Containers and Volumes

This setup consists of the following containers:

- **server**: The PBS Pro server responsible for job scheduling and resource management. Also serves as a login node.
- **p1, p2**: Compute nodes (running `pbs_mom`).

### Persistent Volumes:

- `pbs_home`: Mounted to `/var/spool/pbs`
- `data`: Mounted to `/data`

### Users

There is a single user beside root called `pbsuser`.
Submitting to the queue should happen through it since PBS doesn't allow root submission: `docker exec -u pbsuser...`


## 🛠️  Building the Docker Image

The version of the PBS Pro project and the Docker build process can be simplified
by using a `.env` file, which will be automatically picked up by Docker Compose.

Update the `PBS_TAG` and `IMAGE_TAG` found in the `.env` file and build
the image:

```bash
docker compose build
```

Alternatively, you can build the PBS Pro Docker image locally by specifying the
[PBS_TAG](https://github.com/pbspro/pbspro/releases) as a build argument and
tagging the container with a version ***(IMAGE_TAG)***:

```bash
docker build --build-arg PBS_TAG="v22.05.1" -t pbs-docker-cluster:22.05.1 .
```

## 🚀 Starting the Cluster

Once the image is built, deploy the cluster with the default version of PBS Pro
using Docker Compose:

```bash
docker compose up -d
```

To specify a specific version and override what is configured in `.env`, specify
the `IMAGE_TAG`:

```bash
IMAGE_TAG=22.05.1 docker compose up -d
```

This will start up all containers in detached mode. You can monitor their status using:

```bash
docker compose ps
```

## 📊 Monitoring and Interaction

For real-time cluster logs, use:

```bash
docker compose logs -f
```

To interact with the PBS server, you can execute commands in the server container:

```bash
docker exec -it pbs-server bash
```

To check PBS server status:

```bash
docker exec -it pbs-server qstat -Q
```

To check compute nodes status:

```bash
docker exec -it pbs-server pbsnodes -a
```

## 🔒 Security Notes

This setup is designed for development and testing purposes. For production use, consider:

- Securing the PBS configuration files
- Setting strong credentials for service accounts
- Implementing network policies for container communication
- Using persistent volumes with appropriate permissions

## 📝 How to run tests with PBS

For setup see above.
In short:

- install docker
- From `infra/pbs_local`
  - build the containers with `docker compose build`
  - ensure the containers are running `docker compose up -d`
  - This will mount `~/.tierkreis` and `~/.psij` inside the containers on `/home/pbsuser/` and the tierkreis directory to `/tierkreis`
  - To interact with the PBS cluster you can `docker exec -it pbs-server bash`

To run MPI-based tests, you can submit jobs using PBS commands or use the provided
worker implementation. For more information, see the test files in the main tierkreis repository.

**Caveats**:

- MPI workers behavior may vary based on PBS job allocation
- Initial cluster setup may take a few seconds for all daemons to initialize
