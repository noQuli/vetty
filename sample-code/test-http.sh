#!/bin/sh
echo "=== Testing HTTP via proxy ==="
curl -sv http://httpbin.org/get 2>&1
echo ""
echo "=== Testing HTTPS via proxy ==="
curl -sv https://httpbin.org/get 2>&1
echo "=== Done ==="
