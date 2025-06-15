#!/bin/bash

# Rigorous test script for HyperProcessor RASP

# Exit immediately if a command exits with a non-zero status.
set -e
# Treat unset variables as an error when substituting.
# set -u # Temporarily disabled as HYPER_RASP_CONFIG_PATH might be unset intentionally
# Pipefail: return value of a pipeline is the status of the last command to exit with a non-zero status,
# or zero if no command exited with a non-zero status
set -o pipefail

# --- Configuration ---
RASP_LIB_FILENAME="libhyper_processor.so"
RASP_LIB_DIR="./target/debug" # Relative to project root (hyper_processor/)
RASP_LIB_PATH="$RASP_LIB_DIR/$RASP_LIB_FILENAME"

# Ensure we are in the hyper_processor directory
if [[ "$(basename "$PWD")" != "hyper_processor" ]]; then
  if [[ -d "hyper_processor" ]]; then
    echo "Changing directory to hyper_processor/"
    cd hyper_processor
  else
    echo "ERROR: This script must be run from the root of the 'cooooodez' workspace or from the 'hyper_processor' directory."
    echo "Current directory: $PWD"
    exit 1
  fi
fi

# Test artifacts directory (relative to hyper_processor/)
TEST_ARTIFACTS_DIR="./test_artifacts"
LOG_DIR="$TEST_ARTIFACTS_DIR/logs"
DUMMY_LIB_DIR="$TEST_ARTIFACTS_DIR/dummy_libs"
CONFIG_DIR="$TEST_ARTIFACTS_DIR/configs"
CURRENT_TEST_OUTPUT="$LOG_DIR/current_test_output.txt" # Temporary file for current test logs

# Test counters
tests_run=0
tests_passed=0
tests_failed=0

# Log levels for RASP (RUST_LOG)
LOG_LEVEL_INFO="hyper_processor=info,info" # Default for tests

# --- Helper Functions ---

# Print a section header
print_header() {
  echo ""
  echo "========================================================================"
  echo "= $1"
  echo "========================================================================"
  echo ""
}

# Function to be called on script exit
cleanup() {
  print_header "Cleaning up test artifacts"
  if [ -d "$TEST_ARTIFACTS_DIR" ]; then
    echo "Removing $TEST_ARTIFACTS_DIR..."
    rm -rf "$TEST_ARTIFACTS_DIR"
  else
    echo "$TEST_ARTIFACTS_DIR does not exist, no cleanup needed for it."
  fi
  # Unset any environment variables we might have set
  unset HYPER_RASP_AUDIT_MODE
  unset HYPER_RASP_WHITELIST
  unset HYPER_RASP_CONFIG_PATH
  echo "Cleanup complete."
}

# Trap EXIT signal to call cleanup function
trap cleanup EXIT

# Build RASP library
build_rasp() {
  print_header "Building HyperProcessor RASP Library"
  if cargo build; then # Using debug build for tests for faster compilation
    echo "RASP library built successfully."
  else
    echo "ERROR: RASP library build failed!"
    exit 1
  fi
  if [ ! -f "$RASP_LIB_PATH" ]; then
    echo "ERROR: $RASP_LIB_PATH not found after build! Check RASP_LIB_DIR and build profile."
    exit 1
  fi
}

# Build a dummy shared library from C source
# $1: library name (e.g., "unauth_lib")
# $2: C source code content (optional, defaults to a simple function)
build_dummy_lib() {
  local lib_name="$1"
  local c_content="${2:-void ${lib_name}_func(void) { /* Dummy function for ${lib_name} */ }}"
  local c_file="$DUMMY_LIB_DIR/${lib_name}.c"
  local so_file="$DUMMY_LIB_DIR/${lib_name}.so"

  echo "Building dummy library: $lib_name.so"
  echo -e "$c_content" > "$c_file"
  if gcc -shared -fPIC -o "$so_file" "$c_file"; then
    echo "Dummy library $lib_name.so built successfully."
  else
    echo "ERROR: Failed to build dummy library $lib_name.so!"
    cat "$c_file"
    exit 1
  fi
}

# Initialize test environment
init_test_env() {
  print_header "Initializing Test Environment"
  
  if [ -d "$TEST_ARTIFACTS_DIR" ]; then
    echo "Removing old $TEST_ARTIFACTS_DIR..."
    rm -rf "$TEST_ARTIFACTS_DIR"
  fi
  mkdir -p "$LOG_DIR"
  mkdir -p "$DUMMY_LIB_DIR"
  mkdir -p "$CONFIG_DIR"
  echo "Test artifacts directory created: $TEST_ARTIFACTS_DIR"

  build_rasp
  build_dummy_lib "unauth_A"
  build_dummy_lib "unauth_B"
  build_dummy_lib "whitelisted_A"
  build_dummy_lib "whitelisted_B"
  
  echo "Test environment initialized."
}

# Run a test case
# $1: Test description
# $2: Command to run (can include PRELOAD_WITH_ placeholders for dummy libs)
# $3: Expected exit code of the application being run (e.g., ls, true)
# $4: Expected RASP behavior code (0=RASP allows, app runs normally; 1=RASP terminates process; 2=RASP allows in audit mode, app runs normally)
# $5: Grep pattern to find in RASP logs (e.g., "SECURITY_ALERT.*unauth_A.so") or "" if no specific log check
# $6: Whether the grep pattern should be found (true) or not found (false)
run_test() {
  local description="$1"
  local app_command_template="$2" # May contain PRELOAD_WITH_ placeholders
  local expected_app_exit_code="$3"
  local expected_rasp_behavior="$4" # 0:ok, 1:terminate, 2:audit_ok
  local grep_pattern="${5:-}"
  local grep_should_find="${6:-true}"

  tests_run=$((tests_run + 1))
  echo ""
  echo "------------------------------------------------------------------------"
  echo "Running Test #$tests_run: $description"
  echo "App Command Template: $app_command_template"
  echo "Expected App Exit (if RASP allows): $expected_app_exit_code, Expected RASP Behavior: $expected_rasp_behavior"
  [ -n "$grep_pattern" ] && echo "Grep Pattern: '$grep_pattern', Should Find: $grep_should_find"
  
  local final_ld_preload="$RASP_LIB_PATH"
  local final_app_command="$app_command_template"

  # Construct LD_PRELOAD string if placeholders are used
  if [[ "$app_command_template" == *PRELOAD_WITH_* ]]; then
      libs_to_add=""
      if [[ "$app_command_template" == *PRELOAD_WITH_UNAUTH_A* ]]; then libs_to_add="$libs_to_add:$DUMMY_LIB_DIR/unauth_A.so"; fi
      if [[ "$app_command_template" == *PRELOAD_WITH_UNAUTH_B* ]]; then libs_to_add="$libs_to_add:$DUMMY_LIB_DIR/unauth_B.so"; fi
      if [[ "$app_command_template" == *PRELOAD_WITH_WHITELISTED_A* ]]; then libs_to_add="$libs_to_add:$DUMMY_LIB_DIR/whitelisted_A.so"; fi
      if [[ "$app_command_template" == *PRELOAD_WITH_WHITELISTED_B* ]]; then libs_to_add="$libs_to_add:$DUMMY_LIB_DIR/whitelisted_B.so"; fi
      
      final_ld_preload="$RASP_LIB_PATH$libs_to_add"
      # Remove placeholders from the command part
      final_app_command="${app_command_template//PRELOAD_WITH_UNAUTH_A/}"
      final_app_command="${final_app_command//PRELOAD_WITH_UNAUTH_B/}"
      final_app_command="${final_app_command//PRELOAD_WITH_WHITELISTED_A/}"
      final_app_command="${final_app_command//PRELOAD_WITH_WHITELISTED_B/}"
      final_app_command=$(echo "$final_app_command" | sed 's/  */ /g' | sed 's/^ *//g' | sed 's/ *$//g') # Compact spaces and trim
      echo "Effective LD_PRELOAD: $final_ld_preload"
  fi

  local full_command_str="LD_PRELOAD='$final_ld_preload' RUST_LOG='$LOG_LEVEL_INFO' $final_app_command"
  echo "Executing: $full_command_str"
  echo "------------------------------------------------------------------------"

  set +e # Temporarily disable exit on error to capture $?
  eval "$full_command_str" > "$CURRENT_TEST_OUTPUT" 2>&1
  local actual_process_exit_code=$?
  set -e

  echo "Process finished with exit code: $actual_process_exit_code"
  # cat "$CURRENT_TEST_OUTPUT" # Uncomment for debugging all output

  local test_passed_flag=true
  local reason=""

  local effective_expected_exit_code
  if [ "$expected_rasp_behavior" -eq 1 ]; then 
    effective_expected_exit_code=1 
  else 
    effective_expected_exit_code="$expected_app_exit_code"
  fi

  if [ "$actual_process_exit_code" -ne "$effective_expected_exit_code" ]; then
    test_passed_flag=false
    reason="Expected process exit $effective_expected_exit_code, got $actual_process_exit_code."
  fi

  if [ "$test_passed_flag" = true ] && [ -n "$grep_pattern" ]; then
    set +e 
    grep -qE "$grep_pattern" "$CURRENT_TEST_OUTPUT"
    local grep_found_code=$?
    set -e

    if [ "$grep_should_find" = true ] && [ "$grep_found_code" -ne 0 ]; then
      test_passed_flag=false
      reason="Grep pattern '$grep_pattern' NOT found in logs, but was expected."
    elif [ "$grep_should_find" = false ] && [ "$grep_found_code" -eq 0 ]; then
      test_passed_flag=false
      reason="Grep pattern '$grep_pattern' FOUND in logs, but was NOT expected."
    fi
  fi

  if [ "$test_passed_flag" = true ]; then
    tests_passed=$((tests_passed + 1))
    echo "TEST #$tests_run: $description -- PASSED"
  else
    tests_failed=$((tests_failed + 1))
    echo "TEST #$tests_run: $description -- FAILED"
    echo "  Reason: $reason"
    echo "  Log output ($CURRENT_TEST_OUTPUT) for failed test:"
    cat "$CURRENT_TEST_OUTPUT" | sed 's/^/    /'
  fi
  echo "------------------------------------------------------------------------"
}

# --- Test Scenarios ---

init_test_env # Initialize the environment

print_header "STARTING TEST SCENARIOS"

# --- Basic Functionality Tests ---
run_test "Basic RASP Load (ls)" \
  "ls /tmp" \
  0 \
  0 # RASP OK, ls OK

run_test "Basic RASP Load (true command)" \
  "true" \
  0 \
  0 # RASP OK, true OK

# --- Unauthorized Library Detection (Default: Terminate Mode) ---
unset HYPER_RASP_AUDIT_MODE HYPER_RASP_CONFIG_PATH HYPER_RASP_WHITELIST # Clear env for default behavior

run_test "Unauthorized Lib (unauth_A) - Default Terminate" \
  "PRELOAD_WITH_UNAUTH_A ls /tmp" \
  0 \
  1 \
  "SECURITY_ALERT.*unauth_A.so" \
  true

run_test "Two Unauthorized Libs (unauth_A, unauth_B) - Default Terminate" \
  "PRELOAD_WITH_UNAUTH_A PRELOAD_WITH_UNAUTH_B ls /tmp" \
  0 \
  1 \
  "SECURITY_ALERT.*(unauth_A.so|unauth_B.so)" \
  true

# --- Audit Mode Tests (via Environment Variable) ---
export HYPER_RASP_AUDIT_MODE=true

run_test "Unauthorized Lib (unauth_A) - Audit Mode (Env)" \
  "PRELOAD_WITH_UNAUTH_A ls /tmp" \
  0 \
  2 \
  "AUDIT_ALERT.*unauth_A.so" \
  true

run_test "Two Unauthorized Libs (unauth_A, unauth_B) - Audit Mode (Env)" \
  "PRELOAD_WITH_UNAUTH_A PRELOAD_WITH_UNAUTH_B ls /tmp" \
  0 \
  2 \
  "AUDIT_ALERT.*unauth_B.so" \
  true

unset HYPER_RASP_AUDIT_MODE

# --- Whitelisting Tests (via Environment Variable) ---
export HYPER_RASP_WHITELIST="whitelisted_A.so,whitelisted_B.so"

run_test "Whitelisted Lib (whitelisted_A) - Allowed (Env)" \
  "PRELOAD_WITH_WHITELISTED_A ls /tmp" \
  0 \
  0 \
  "whitelisted_A.so" \
  false # Should NOT find an ALERT for whitelisted_A

run_test "Whitelisted (A) & Unauthorized (unauth_A) - Terminate (Env Whitelist)" \
  "PRELOAD_WITH_WHITELISTED_A PRELOAD_WITH_UNAUTH_A ls /tmp" \
  0 \
  1 \
  "SECURITY_ALERT.*unauth_A.so" \
  true

export HYPER_RASP_AUDIT_MODE=true 
run_test "Whitelisted (A,B) & Unauthorized (unauth_A) - Audit (Env Whitelist)" \
  "PRELOAD_WITH_WHITELISTED_A PRELOAD_WITH_WHITELISTED_B PRELOAD_WITH_UNAUTH_A ls /tmp" \
  0 \
  2 \
  "AUDIT_ALERT.*unauth_A.so" \
  true
run_test "Whitelisted (A,B) & Unauthorized (unauth_A) - Audit (Env Whitelist) - No Alert for Whitelisted_B" \
  "PRELOAD_WITH_WHITELISTED_A PRELOAD_WITH_WHITELISTED_B PRELOAD_WITH_UNAUTH_A ls /tmp" \
  0 \
  2 \
  "whitelisted_B.so" \
  false 

unset HYPER_RASP_WHITELIST HYPER_RASP_AUDIT_MODE

# --- Config File Tests ---
print_header "CONFIG FILE TESTS"
CONFIG_FILE_1="$CONFIG_DIR/config1.yaml"
echo -e "audit_mode: true\nwhitelisted_filenames:\n  - whitelisted_A.so" > "$CONFIG_FILE_1"
export HYPER_RASP_CONFIG_PATH="$CONFIG_FILE_1"

run_test "Config File 1: Audit=true, WL=A.so -> Unauth B triggers AUDIT" \
  "PRELOAD_WITH_WHITELISTED_A PRELOAD_WITH_UNAUTH_B ls /tmp" \
  0 \
  2 \
  "AUDIT_ALERT.*unauth_B.so" \
  true

run_test "Config File 1: Audit=true, WL=A.so -> Whitelisted A is CLEAN" \
  "PRELOAD_WITH_WHITELISTED_A PRELOAD_WITH_UNAUTH_B ls /tmp" \
  0 \
  2 \
  "whitelisted_A.so" \
  false 

unset HYPER_RASP_CONFIG_PATH

# Config File Override by Env Vars
CONFIG_FILE_2="$CONFIG_DIR/config2.yaml"
echo -e "audit_mode: true\nwhitelisted_filenames:\n  - \"whitelisted_A.so\"" > "$CONFIG_FILE_2"
export HYPER_RASP_CONFIG_PATH="$CONFIG_FILE_2"
export HYPER_RASP_AUDIT_MODE=false 
export HYPER_RASP_WHITELIST="whitelisted_B.so" 

run_test "CFG2+Env: Audit=false(Env), WL=B.so(Env) -> Unauth A triggers SECURITY" \
  "PRELOAD_WITH_UNAUTH_A PRELOAD_WITH_WHITELISTED_B ls /tmp" \
  0 \
  1 \
  "SECURITY_ALERT.*unauth_A.so" \
  true

run_test "CFG2+Env: Audit=false(Env), WL=B.so(Env) -> Whitelisted A (from file, but overridden by Env WL) is UNAUTH -> SECURITY" \
  "PRELOAD_WITH_WHITELISTED_A PRELOAD_WITH_WHITELISTED_B ls /tmp" \
  0 \
  1 \
  "SECURITY_ALERT.*whitelisted_A.so" \
  true
  
run_test "CFG2+Env: Audit=false(Env), WL=B.so(Env) -> Whitelisted B (from Env) is CLEAN" \
  "PRELOAD_WITH_WHITELISTED_B ls /tmp" \
  0 \
  0 \
  "whitelisted_B.so" \
  false

unset HYPER_RASP_CONFIG_PATH HYPER_RASP_AUDIT_MODE HYPER_RASP_WHITELIST

# --- Summary ---
print_header "TEST SUMMARY"
echo "Total tests run: $tests_run"
echo "Passed: $tests_passed"
echo "Failed: $tests_failed"
echo ""

if [ "$tests_failed" -gt 0 ]; then
  echo "CI STATUS: FAILED ($tests_failed tests failed)"
  exit 1
else
  echo "CI STATUS: PASSED (All $tests_passed tests passed)"
  exit 0
fi 