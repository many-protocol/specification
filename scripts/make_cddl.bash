#!/usr/bin/env bash

find_cddl() {
    local i

    # List all in directory order (breadth-first), and remove the files
    # that aren't part of the repository from the list.
    find "$1" -maxdepth 0 -iname \*.cddl | sort -n
    find "*/$1" -maxdepth 0 -iname \*.cddl | sort -n || true

}

{
    find_cddl "$(dirname "$(dirname "$0")")"/spec #| xargs -0 echo
    find_cddl "$(dirname "$(dirname "$0")")"/attributes #| xargs -0 echo
} > "$1"
