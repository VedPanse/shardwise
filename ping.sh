curl -X POST http://localhost:8080/jobs \
  -H "Content-Type: application/json" \
  -d '{"task": "sum_numbers", "input": {"numbers": [1, 2, 3, 4]}}'