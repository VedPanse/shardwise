#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
state_file="$script_dir/.shardwise/last-job.json"
api_host="${SHARDWISE_URL:-http://localhost:8080}"
command="${1:-submit}"
if [ "$#" -gt 0 ]; then shift; fi

usage() {
    bold=''; accent=''; muted=''; reset=''
    if [ -t 1 ] && [ "${TERM:-dumb}" != dumb ] && [ "${NO_COLOR+x}" != x ]; then
        bold=$(printf '\033[1m')
        accent=$(printf '\033[36m')
        muted=$(printf '\033[2m')
        reset=$(printf '\033[0m')
    fi

    printf '\n  %sSHARDWISE%s  %s/ API command catalogue%s\n' "$bold" "$reset" "$muted" "$reset"
    printf '  Submit a graph. Track its progress.\n'
    printf '\n  %sUSAGE%s\n' "$accent" "$reset"
    printf '    ./ping.sh <command> [argument]\n'
    printf '\n  %sCOMMANDS%s\n' "$accent" "$reset"
    printf '    %ssubmit%s [file.json]\n' "$bold" "$reset"
    printf '      Submit a JSON file, or the built-in 11-task sales pipeline.\n'
    printf '      Four data shards: clean -> summarize -> merge -> report.\n'
    printf '      Remembers the job ID and server after a successful response.\n'
    printf '\n    %sstatus%s [job-id]\n' "$bold" "$reset"
    printf '      Show the last submitted job, or look up a specific ID.\n'
    printf '\n    %shealth%s\n' "$bold" "$reset"
    printf '      Check whether the coordinator is responding.\n'
    printf '\n    %shelp%s\n' "$bold" "$reset"
    printf '      Show this catalogue. Also accepts -h and --help.\n'
    printf '\n  %sQUICK START%s\n' "$accent" "$reset"
    printf '    ./ping.sh                      # Submit the example\n'
    printf '    ./ping.sh status               # Check its status\n'
    printf '    ./ping.sh submit graph.json    # Submit your own graph\n'
    printf '    ./ping.sh status JOB_ID        # Look up another job\n'
    printf '\n  %sSERVER%s\n' "$accent" "$reset"
    printf '    Default: http://localhost:8080\n'
    printf '    Configured: %s\n' "$api_host"
    printf '    SHARDWISE_URL=http://localhost:9090 ./ping.sh submit\n'
    printf '    ./ping.sh status uses the saved server when no ID is given.\n'
    printf '\n  %sLOCAL STATE%s\n' "$accent" "$reset"
    printf '    .shardwise/last-job.json beside this script; ignored by Git.\n'
    printf '    No command defaults to submit. Requires curl and Python 3.\n'
    printf '    Set NO_COLOR=1 to disable colors.\n\n'
}

case "$command" in
    -h|--help|help) usage; exit 0 ;;
    http://*|https://*) api_host="$command"; command=submit ;;
esac
api_host="${api_host%/}"

case "$command" in
    submit)
        if [ "$#" -gt 1 ]; then usage >&2; exit 2; fi
        command -v python3 >/dev/null 2>&1 || {
            echo 'python3 is required to remember the submitted job.' >&2
            exit 1
        }
        if [ "$#" -eq 1 ]; then
            response=$(curl --fail-with-body --silent --show-error \
                --request POST "$api_host/jobs" \
                --header 'Content-Type: application/json' \
                --data-binary "@$1") || { printf '%s\n' "$response" >&2; exit 1; }
        else
            response=$(curl --fail-with-body --silent --show-error \
                --request POST "$api_host/jobs" \
                --header 'Content-Type: application/json' \
                --data-binary @- <<'JSON'
{
  "tasks": [
    {
      "id": "clean_shard_01",
      "function": "clean_orders",
      "args": {
        "orders": {
          "value": [
            {
              "order_id": "ORD-001",
              "quantity": 2,
              "unit_price": 12.5,
              "status": "paid"
            },
            {
              "order_id": "ORD-002",
              "quantity": 1,
              "unit_price": 30,
              "status": "cancelled"
            }
          ]
        },
        "keep_status": {
          "value": "paid"
        }
      }
    },
    {
      "id": "clean_shard_02",
      "function": "clean_orders",
      "args": {
        "orders": {
          "value": [
            {
              "order_id": "ORD-003",
              "quantity": 3,
              "unit_price": 8,
              "status": "paid"
            },
            {
              "order_id": "ORD-004",
              "quantity": 1,
              "unit_price": 45,
              "status": "paid"
            }
          ]
        },
        "keep_status": {
          "value": "paid"
        }
      }
    },
    {
      "id": "clean_shard_03",
      "function": "clean_orders",
      "args": {
        "orders": {
          "value": [
            {
              "order_id": "ORD-005",
              "quantity": 4,
              "unit_price": 6.25,
              "status": "paid"
            },
            {
              "order_id": "ORD-006",
              "quantity": 2,
              "unit_price": 15,
              "status": "cancelled"
            }
          ]
        },
        "keep_status": {
          "value": "paid"
        }
      }
    },
    {
      "id": "clean_shard_04",
      "function": "clean_orders",
      "args": {
        "orders": {
          "value": [
            {
              "order_id": "ORD-007",
              "quantity": 1,
              "unit_price": 60,
              "status": "paid"
            },
            {
              "order_id": "ORD-008",
              "quantity": 2,
              "unit_price": 10,
              "status": "paid"
            }
          ]
        },
        "keep_status": {
          "value": "paid"
        }
      }
    },
    {
      "id": "summarize_shard_01",
      "function": "summarize_sales",
      "args": {
        "orders": {
          "from_task": "clean_shard_01"
        }
      }
    },
    {
      "id": "summarize_shard_02",
      "function": "summarize_sales",
      "args": {
        "orders": {
          "from_task": "clean_shard_02"
        }
      }
    },
    {
      "id": "summarize_shard_03",
      "function": "summarize_sales",
      "args": {
        "orders": {
          "from_task": "clean_shard_03"
        }
      }
    },
    {
      "id": "summarize_shard_04",
      "function": "summarize_sales",
      "args": {
        "orders": {
          "from_task": "clean_shard_04"
        }
      }
    },
    {
      "id": "merge_shards_01_02",
      "function": "merge_sales_summaries",
      "args": {
        "summary_a": {
          "from_task": "summarize_shard_01"
        },
        "summary_b": {
          "from_task": "summarize_shard_02"
        }
      }
    },
    {
      "id": "merge_shards_03_04",
      "function": "merge_sales_summaries",
      "args": {
        "summary_a": {
          "from_task": "summarize_shard_03"
        },
        "summary_b": {
          "from_task": "summarize_shard_04"
        }
      }
    },
    {
      "id": "sales_report",
      "function": "build_sales_report",
      "args": {
        "summary_a": {
          "from_task": "merge_shards_01_02"
        },
        "summary_b": {
          "from_task": "merge_shards_03_04"
        },
        "currency": {
          "value": "USD"
        }
      }
    }
  ],
  "outputs": [
    "sales_report"
  ]
}
JSON
            ) || { printf '%s\n' "$response" >&2; exit 1; }
        fi
        printf '%s\n' "$response"
        printf '%s' "$response" | python3 -c '
import json, os, pathlib, sys, tempfile
try:
    response = json.load(sys.stdin)
    job_id = response.get("job_id")
    if not isinstance(job_id, str) or not job_id:
        raise ValueError("response has no valid job_id")
    path = pathlib.Path(sys.argv[1])
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", dir=path.parent, delete=False) as f:
            temporary = f.name
            json.dump({"job_id": job_id, "host": sys.argv[2]}, f)
        os.replace(temporary, path)
    finally:
        if temporary and os.path.exists(temporary):
            os.unlink(temporary)
except (ValueError, OSError, AttributeError) as error:
    sys.exit(f"Could not remember submitted job: {error}")
' "$state_file" "$api_host"
        ;;
    status)
        if [ "$#" -gt 1 ]; then usage >&2; exit 2; fi
        url=$(python3 -c '
import json, pathlib, sys
from urllib.parse import quote
try:
    if len(sys.argv) == 4:
        host, job_id = sys.argv[2], sys.argv[3]
    else:
        path = pathlib.Path(sys.argv[1])
        if not path.exists():
            sys.exit("No saved job. Run ./ping.sh submit first, or ./ping.sh status JOB_ID.")
        state = json.loads(path.read_text())
        host, job_id = state["host"], state["job_id"]
    if not isinstance(host, str) or not host.startswith(("http://", "https://")):
        raise ValueError("invalid saved host")
    if not isinstance(job_id, str) or not job_id:
        raise ValueError("invalid job ID")
    print(host.rstrip("/") + "/jobs/" + quote(job_id, safe=""))
except (ValueError, OSError, KeyError, TypeError) as error:
    sys.exit(f"Could not read job: {error}")
' "$state_file" "$api_host" "$@")
        curl --fail-with-body --silent --show-error "$url"
        printf '\n'
        ;;
    health)
        if [ "$#" -ne 0 ]; then usage >&2; exit 2; fi
        curl --fail-with-body --silent --show-error "$api_host/health"
        printf '\n'
        ;;
    *) usage >&2; exit 2 ;;
esac
