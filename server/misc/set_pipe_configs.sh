#!/bin/bash
curl -v \
    -XPOST \
    --header "Content-Type: application/json" \
    --data '{"configs":[{"id":1, "pipe": {"section": [{"name": "sqlite", "path": "/tmp/test.sqlite", "query": "select * from test"},{"endpoint":"http://localhost:7777/ingestion","name":"mycelial_net","token":"mycelial_net_token"}]}}]}' \
    http://localhost:7777/pipe/configs

echo
