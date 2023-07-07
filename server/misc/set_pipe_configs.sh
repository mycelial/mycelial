#!/bin/bash
curl -v \
    -XPOST \
    --header "Content-Type: application/json" \
    --data '{"configs":[{"id":1, "raw_config": "{\"section\":[{\"name\": \"sqlite\", \"path\": \"/tmp/test.sqlite\", \"query\": \"select * from test\"},{\"endpoint\":\"http://localhost:8080/ingestion\",\"name\":\"mycelial_net\",\"token\":\"mycelial_net_token\"}]}"}]}' \
    http://localhost:8080/pipe/configs
