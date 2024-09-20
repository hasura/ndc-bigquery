# ndc-bigquery-cli

ndc-bigquery-cli is used to configure a deployment of ndc-bigquery.
It is intended to be automatically downloaded and invoked via the Hasura CLI, as a plugin.

## Create a configuration

Create a configuration in a new directory using the following commands:

1. Initialize a configuration:

   ```sh
   export HASURA_BIGQUERY_SERVICE_KEY='<bigquery-service-key>'
   export HASURA_BIGQUERY_PROJECT_ID='<bigquery-project-id>'
   export HASURA_BIGQUERY_DATASET_ID='<bigquery-dataset-id>'
   cargo run --bin ndc-bigquery-cli -- --context='<directory>' initialize
   ```

2. Update the configuration by introspecting the database:

   ```sh
   export HASURA_BIGQUERY_SERVICE_KEY='<bigquery-service-key>'
   export HASURA_BIGQUERY_PROJECT_ID='<bigquery-project-id>'
   export HASURA_BIGQUERY_DATASET_ID='<bigquery-dataset-id>'
   cargo run --bin ndc-bigquery-cli -- --context='<directory>' update
   ```
