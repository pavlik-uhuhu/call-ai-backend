# call-ai backend monorepo

Services requires postgresql instance running locally to be able to run tests, easiest way is 
to install [postgresql.app](https://postgresapp.com/) and export env variable as follows:
export DATABASE_URL="postgres://{username}:{password}@127.0.0.1:{db_port}/call-ai"

Queries changes (inside api-server/protocol/worker directory) in code requires sqlx data to be 
updated for project to compile in offline mode (without postgresql instance).

```bash
export DATABASE_URL="postgres://{username}:{password}@127.0.0.1:{db_port}/call-ai"
cargo sqlx database create
cargo sqlx migrate run
cargo sqlx prepare -- --all-features 
```

Manual deploy services to k8s requires next steps (should be automated in CI/CD pipeline):
1. increment Cargo.toml crate version of service (v0.1.x)
2. build and push image of service:
```bash
docker buildx build --platform linux/amd64 -t artifactory.xxx/docker-registry/{service-name}:v0.1.x -f {service-name}/docker/Dockerfile .
docker push artifactory.xxx/docker-registry/{service-name}:v0.1.x
```
3. if update requires new DB migration (applied only to api-server):
```bash
sqlx migrate add -r {migration_name}
docker buildx build --platform linux/amd64 -t artifactory.xxx/docker-registry/api-server-migration:v0.1.x -f api-server/docker/migration.dockerfile .
docker push artifactory.xxx/docker-registry/api-server-migration:v0.1.x
```
4. update /deploy/Chart.yaml appVersion with related version of service
5. update /deploy/values-dev.yaml with new version of image tag/migrationTag
6. inside /deploy directory of service:
```bash
helm upgrade -f values-dev.yaml {sevice-name} . --namespace dev
```
