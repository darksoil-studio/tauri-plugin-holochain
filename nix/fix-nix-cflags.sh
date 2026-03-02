# Move -isystem/-idirafter flags from NIX_CFLAGS_COMPILE to C_INCLUDE_PATH.
#
# The Nix gcc-wrapper passes NIX_CFLAGS_COMPILE as command-line args to gcc,
# causing the flags to be counted twice in exec() (as args + env), which can
# exceed the kernel's ARG_MAX limit ("Argument list too long").
#
# C_INCLUDE_PATH is equivalent to -isystem but is only read from the
# environment by gcc, avoiding the doubling.
#
# This script must be sourced, not executed.

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
export NIX_CFLAGS_COMPILE="$_other_flags"
unset _include_dirs _other_flags _prev _flag
