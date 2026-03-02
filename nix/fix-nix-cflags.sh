# Reduce the size of NIX_CFLAGS_COMPILE and NIX_LDFLAGS to prevent
# "Argument list too long" errors.
#
# Two issues compound:
# 1. The gcc-wrapper passes NIX_CFLAGS_COMPILE as command-line args to gcc,
#    so the content is counted twice in exec() (as args + env).
# 2. Nix doesn't deduplicate flags from overlapping inputsFrom shells.
#
# Fix: move -isystem flags to C_INCLUDE_PATH (avoids doubling) and
# deduplicate all remaining flags.
#
# This script must be sourced, not executed.

# --- Move -isystem/-idirafter to C_INCLUDE_PATH ---
_include_dirs=""
_other_flags=""
_prev=""
for _flag in $NIX_CFLAGS_COMPILE; do
  if [ "$_prev" = "-isystem" ] || [ "$_prev" = "-idirafter" ]; then
    _include_dirs="${_include_dirs:+$_include_dirs:}$_flag"
    _prev=""
  elif [ "$_flag" = "-isystem" ] || [ "$_flag" = "-idirafter" ]; then
    _prev="$_flag"
  else
    _other_flags="$_other_flags $_flag"
  fi
done
export C_INCLUDE_PATH="${C_INCLUDE_PATH:+$C_INCLUDE_PATH:}$_include_dirs"
export CPLUS_INCLUDE_PATH="${CPLUS_INCLUDE_PATH:+$CPLUS_INCLUDE_PATH:}$_include_dirs"

# --- Deduplicate remaining NIX_CFLAGS_COMPILE ---
declare -A _seen=()
_deduped=""
for _flag in $_other_flags; do
  if [ -z "${_seen[$_flag]+x}" ]; then
    _seen[$_flag]=1
    _deduped="$_deduped $_flag"
  fi
done
export NIX_CFLAGS_COMPILE="$_deduped"

# --- Deduplicate NIX_LDFLAGS (handle -rpath pairs) ---
unset _seen
declare -A _seen=()
_deduped=""
_prev=""
for _flag in $NIX_LDFLAGS; do
  if [ "$_prev" = "-rpath" ]; then
    _pair="$_prev $_flag"
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
export NIX_LDFLAGS="$_deduped"

unset _include_dirs _other_flags _prev _flag _seen _deduped _pair
