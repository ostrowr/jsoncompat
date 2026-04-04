#!/usr/bin/env bash

# End-to-end CLI demo and smoke test.
#
# The demo prints each schema fixture, runs the matching CLI command, and shows
# the resulting output. By default it pauses before each section; pass `-n` or
# `--noninteractive` for CI.

set -Eeuo pipefail

readonly repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly temp_dir="$(mktemp -d)"
readonly command_output_path="${temp_dir}/command-output.txt"

interactive=1
step_index=0
jsoncompat_bin=""

cleanup() {
  rm -rf "${temp_dir}"
}
trap cleanup EXIT

if [[ -z "${NO_COLOR:-}" ]]; then
  readonly c_reset=$'\033[0m'
  readonly c_dim=$'\033[2m'
  readonly c_blue=$'\033[1;34m'
  readonly c_cyan=$'\033[1;36m'
  readonly c_green=$'\033[1;32m'
  readonly c_magenta=$'\033[1;35m'
  readonly c_red=$'\033[1;31m'
  readonly c_yellow=$'\033[1;33m'
else
  readonly c_reset=""
  readonly c_dim=""
  readonly c_blue=""
  readonly c_cyan=""
  readonly c_green=""
  readonly c_magenta=""
  readonly c_red=""
  readonly c_yellow=""
fi

usage() {
  cat <<'EOF'
Usage: scripts/demo.sh [--noninteractive|-n]

Options:
  -n, --noninteractive   Run without pausing between steps.
  -h, --help             Show this help text.
EOF
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --)
        shift
        ;;
      -n | --noninteractive)
        interactive=0
        shift
        ;;
      -h | --help)
        usage
        exit 0
        ;;
      *)
        printf '%sunknown argument: %s%s\n' "${c_red}" "$1" "${c_reset}" >&2
        usage >&2
        exit 2
        ;;
    esac
  done
}

section() {
  step_index=$((step_index + 1))
  printf '\n%s[%02d] %s%s\n' "${c_magenta}" "${step_index}" "$1" "${c_reset}"
  printf '%sExpected:%s %s\n' "${c_dim}" "${c_reset}" "$2"

  if [[ "${interactive}" -eq 0 ]]; then
    return
  fi
  if [[ ! -t 0 ]]; then
    printf '%sinteractive mode requires a TTY; rerun with --noninteractive%s\n' "${c_red}" "${c_reset}" >&2
    exit 2
  fi

  printf '%sPress Enter to continue (q + Enter to quit): %s' "${c_blue}" "${c_reset}"
  local reply=""
  IFS= read -r reply
  if [[ "${reply}" == "q" || "${reply}" == "Q" ]]; then
    printf '%sDemo stopped before running this section.%s\n' "${c_yellow}" "${c_reset}"
    exit 0
  fi
}

subsection() {
  printf '\n%s-- %s --%s\n' "${c_blue}" "$1" "${c_reset}"
}

log_command() {
  printf '%s$%s' "${c_cyan}" "${c_reset}"
  for arg in "$@"; do
    printf ' %q' "${arg}"
  done
  printf '\n'
}

print_json_file() {
  local path="$1"

  if command -v jq >/dev/null 2>&1; then
    jq -C . "${path}"
  else
    cat "${path}"
  fi
}

show_schema() {
  subsection "$1"
  print_json_file "$2"
}

show_output() {
  subsection "Output"
  if [[ "$1" == "json" ]]; then
    print_json_file "${command_output_path}"
  else
    cat "${command_output_path}"
  fi
}

run_command() {
  local expected_status="$1"
  local output_format="$2"
  shift 2

  subsection "Command"
  log_command "$@"

  if "$@" >"${command_output_path}" 2>&1; then
    if [[ "${expected_status}" == "fail" ]]; then
      cat "${command_output_path}"
      printf '%sexpected this command to fail, but it succeeded%s\n' "${c_red}" "${c_reset}" >&2
      return 1
    fi
    show_output "${output_format}"
    printf '%s✔ command succeeded%s\n' "${c_green}" "${c_reset}"
    return
  fi

  if [[ "${expected_status}" == "ok" ]]; then
    cat "${command_output_path}"
    return 1
  fi

  show_output "${output_format}"
  printf '%s✔ expected failure observed%s\n' "${c_yellow}" "${c_reset}"
}

write_fixture() {
  local path="$1"
  shift
  cat >"${path}" <<EOF
$*
EOF
}

create_fixtures() {
  write_fixture "${temp_dir}/compat-old.json" '{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "integer",
  "minimum": 0,
  "maximum": 10
}'

  write_fixture "${temp_dir}/compat-new.json" '{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "integer",
  "minimum": 2,
  "maximum": 8
}'

  write_fixture "${temp_dir}/incompat-old.json" '{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "string",
  "minLength": 2
}'

  write_fixture "${temp_dir}/incompat-new.json" '{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "integer",
  "minimum": 1
}'

  write_fixture "${temp_dir}/sat-schema.json" '{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "id": { "type": "integer", "minimum": 1 },
    "tags": {
      "type": "array",
      "items": { "type": "string", "minLength": 1 },
      "contains": { "pattern": "^(?:prod|dev)$" },
      "minContains": 1,
      "minItems": 1,
      "maxItems": 3,
      "uniqueItems": true
    }
  },
  "required": ["id", "tags"],
  "additionalProperties": false
}'

  write_fixture "${temp_dir}/unsat-schema.json" '{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "array",
  "minItems": 2,
  "maxItems": 1
}'

  write_fixture "${temp_dir}/golden-old.json" '{
  "compatible_field": {
    "mode": "serializer",
    "stable_id": "compatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "integer",
      "minimum": 0
    }
  },
  "incompatible_field": {
    "mode": "serializer",
    "stable_id": "incompatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "string",
      "minLength": 1
    }
  }
}'

  write_fixture "${temp_dir}/golden-compatible.json" '{
  "compatible_field": {
    "mode": "serializer",
    "stable_id": "compatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "integer",
      "minimum": 2
    }
  },
  "incompatible_field": {
    "mode": "serializer",
    "stable_id": "incompatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "string",
      "minLength": 3
    }
  }
}'

  write_fixture "${temp_dir}/golden-incompatible.json" '{
  "compatible_field": {
    "mode": "serializer",
    "stable_id": "compatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "integer",
      "minimum": 2
    }
  },
  "incompatible_field": {
    "mode": "serializer",
    "stable_id": "incompatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "integer",
      "minimum": 1
    }
  }
}'
}

build_cli() {
  section "Build CLI" "compile the local jsoncompat binary"
  run_command ok text cargo build --quiet --bin jsoncompat

  local target_dir="${CARGO_TARGET_DIR:-${repo_root}/target}"
  if [[ "${target_dir}" != /* ]]; then
    target_dir="${repo_root}/${target_dir}"
  fi

  jsoncompat_bin="${target_dir}/debug/jsoncompat"
  if [[ ! -x "${jsoncompat_bin}" && -x "${jsoncompat_bin}.exe" ]]; then
    jsoncompat_bin="${jsoncompat_bin}.exe"
  fi
  if [[ ! -x "${jsoncompat_bin}" ]]; then
    printf '%sjsoncompat binary not found at %s%s\n' "${c_red}" "${jsoncompat_bin}" "${c_reset}" >&2
    exit 1
  fi

  printf '%sUsing %s%s\n' "${c_dim}" "${jsoncompat_bin}" "${c_reset}"
}

run_demo() {
  section "Canonicalize a schema" "show raw input and canonical rewrite"
  show_schema "Raw schema" "${temp_dir}/sat-schema.json"
  run_command ok json "${jsoncompat_bin}" canonicalize "${temp_dir}/sat-schema.json" --pretty

  section "Generate valid sample data" "emit raw-valid JSON instances"
  show_schema "Satisfiable schema" "${temp_dir}/sat-schema.json"
  run_command ok json "${jsoncompat_bin}" generate "${temp_dir}/sat-schema.json" --count 3 --depth 6 --pretty

  section "Reject unsatisfiable schemas" "generation must fail"
  show_schema "Unsatisfiable schema" "${temp_dir}/unsat-schema.json"
  run_command fail text "${jsoncompat_bin}" generate "${temp_dir}/unsat-schema.json" --count 1 --depth 4

  section "Compatible serializer change" "static check should pass"
  show_schema "Old schema" "${temp_dir}/compat-old.json"
  show_schema "New schema" "${temp_dir}/compat-new.json"
  run_command ok text "${jsoncompat_bin}" compat \
    "${temp_dir}/compat-old.json" \
    "${temp_dir}/compat-new.json" \
    --role serializer

  section "Incompatible serializer change" "fuzzed counterexample must fail"
  show_schema "Old schema" "${temp_dir}/incompat-old.json"
  show_schema "New schema" "${temp_dir}/incompat-new.json"
  run_command fail text "${jsoncompat_bin}" compat \
    "${temp_dir}/incompat-old.json" \
    "${temp_dir}/incompat-new.json" \
    --role serializer \
    --fuzz 64 \
    --depth 6

  section "CI grading in JSON mode" "compatible golden set should pass"
  show_schema "Old golden file" "${temp_dir}/golden-old.json"
  show_schema "New golden file" "${temp_dir}/golden-compatible.json"
  run_command ok json "${jsoncompat_bin}" ci \
    "${temp_dir}/golden-old.json" \
    "${temp_dir}/golden-compatible.json" \
    --display json

  section "CI grading in table mode" "incompatible golden set must fail"
  show_schema "Old golden file" "${temp_dir}/golden-old.json"
  show_schema "New golden file" "${temp_dir}/golden-incompatible.json"
  run_command fail text "${jsoncompat_bin}" ci \
    "${temp_dir}/golden-old.json" \
    "${temp_dir}/golden-incompatible.json" \
    --display table
}

main() {
  parse_args "$@"

  cd "${repo_root}"
  printf '%sjsoncompat CLI demo%s\n' "${c_blue}" "${c_reset}"
  printf '%sTemporary fixtures: %s%s\n' "${c_dim}" "${temp_dir}" "${c_reset}"

  create_fixtures
  build_cli
  run_demo

  printf '\n%s✔ demo completed successfully%s\n' "${c_green}" "${c_reset}"
}

main "$@"
