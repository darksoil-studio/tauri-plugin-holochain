# Reduce the size of NIX_CFLAGS_COMPILE* and NIX_LDFLAGS* to prevent
# "Argument list too long" errors.
#
# Three issues compound:
# 1. The gcc-wrapper passes NIX_CFLAGS_COMPILE (and _FOR_BUILD/_FOR_TARGET)
#    as command-line args to gcc, so the content is counted twice in exec()
#    (as args + env).
# 2. Nix doesn't deduplicate flags from overlapping inputsFrom shells.
# 3. The _FOR_BUILD variant is often the largest (38KB+) but was not handled.
#
# Fix: move -isystem flags to C_INCLUDE_PATH (avoids doubling), strip
# -fmacro-prefix-map flags (only for debug info reproducibility, not needed
# in dev shells), and deduplicate everything.
#
# This script must be sourced, not executed.

# --- Helper: reduce a CFLAGS variable ---
# Moves -isystem/-idirafter to C_INCLUDE_PATH, strips -fmacro-prefix-map,
# and deduplicates remaining flags.
# Usage: _reduce_cflags "VAR_NAME"
_reduce_cflags() {
  local _varname="$1"
  local _value="${!_varname}"
  [ -z "$_value" ] && return

  local _include_dirs=""
  local _other_flags=""
  local _prev=""
  local _flag

  for _flag in $_value; do
    if [ "$_prev" = "-isystem" ] || [ "$_prev" = "-idirafter" ]; then
      _include_dirs="${_include_dirs:+$_include_dirs:}$_flag"
      _prev=""
    elif [ "$_flag" = "-isystem" ] || [ "$_flag" = "-idirafter" ]; then
      _prev="$_flag"
    elif [[ "$_flag" == -fmacro-prefix-map=* ]]; then
      : # strip -fmacro-prefix-map (only affects debug info paths)
    else
      _other_flags="$_other_flags $_flag"
    fi
  done

  if [ -n "$_include_dirs" ]; then
    _all_include_dirs="${_all_include_dirs:+$_all_include_dirs:}$_include_dirs"
  fi

  # Deduplicate remaining flags
  declare -A _seen=()
  local _deduped=""
  for _flag in $_other_flags; do
    if [ -z "${_seen[$_flag]+x}" ]; then
      _seen[$_flag]=1
      _deduped="$_deduped $_flag"
    fi
  done

  export "$_varname"="$_deduped"
}

# --- Helper: reduce an LDFLAGS variable ---
# Deduplicates flags, handling -rpath pairs.
# Usage: _reduce_ldflags "VAR_NAME"
_reduce_ldflags() {
  local _varname="$1"
  local _value="${!_varname}"
  [ -z "$_value" ] && return

  declare -A _seen=()
  local _deduped=""
  local _prev=""
  local _flag

  for _flag in $_value; do
    if [ "$_prev" = "-rpath" ]; then
      local _pair="$_prev $_flag"
      if [ -z "${_seen[$_pair]+x}" ]; then
        _seen[$_pair]=1
        _deduped="$_deduped $_prev $_flag"
      fi
      _prev=""
    elif [ "$_flag" = "-rpath" ]; then
      _prev="$_flag"
    else
      if [ -z "${_seen[$_flag]+x}" ]; then
        _seen[$_flag]=1
        _deduped="$_deduped $_flag"
      fi
    fi
  done

  export "$_varname"="$_deduped"
}

# --- Helper: deduplicate a colon-separated path variable ---
_dedup_path() {
  local _varname="$1"
  local _value="${!_varname}"
  [ -z "$_value" ] && return

  declare -A _seen=()
  local _deduped=""
  local _dir

  IFS=: read -ra _dirs <<< "$_value"
  for _dir in "${_dirs[@]}"; do
    [ -z "$_dir" ] && continue
    if [ -z "${_seen[$_dir]+x}" ]; then
      _seen[$_dir]=1
      _deduped="${_deduped:+$_deduped:}$_dir"
    fi
  done

  export "$_varname"="$_deduped"
}

# Collect all include dirs across variants, then deduplicate
_all_include_dirs=""

# Apply to all CFLAGS variants
_reduce_cflags NIX_CFLAGS_COMPILE
_reduce_cflags NIX_CFLAGS_COMPILE_FOR_BUILD
_reduce_cflags NIX_CFLAGS_COMPILE_FOR_TARGET

# Set deduplicated include paths
if [ -n "$_all_include_dirs" ]; then
  export C_INCLUDE_PATH="${C_INCLUDE_PATH:+$C_INCLUDE_PATH:}$_all_include_dirs"
  export CPLUS_INCLUDE_PATH="${CPLUS_INCLUDE_PATH:+$CPLUS_INCLUDE_PATH:}$_all_include_dirs"
  _dedup_path C_INCLUDE_PATH
  _dedup_path CPLUS_INCLUDE_PATH
fi

# Apply to all LDFLAGS variants
_reduce_ldflags NIX_LDFLAGS
_reduce_ldflags NIX_LDFLAGS_FOR_BUILD
_reduce_ldflags NIX_LDFLAGS_FOR_TARGET

unset _all_include_dirs
unset -f _reduce_cflags _reduce_ldflags _dedup_path
