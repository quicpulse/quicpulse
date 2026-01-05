#!/bin/bash
# Test plugin for integration testing
# Reads HookContext JSON from stdin, outputs HookResult JSON to stdout

# Read stdin
INPUT=$(cat)

# Parse the hook name from argument
HOOK="$1"

case "$HOOK" in
  "pre_request")
    # Add a custom header to the request
    echo '{"continue_processing": true, "add_headers": {"X-Plugin-Test": "pre-request-executed"}}'
    ;;
  "post_response")
    # Just pass through
    echo '{"continue_processing": true, "add_headers": {"X-Plugin-Post": "post-response-executed"}}'
    ;;
  *)
    echo '{"continue_processing": true}'
    ;;
esac
