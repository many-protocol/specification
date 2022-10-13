#!/usr/bin/env bash

find_cddl() {
    local i

    # List all in directory order (breadth-first), and remove the files
    # that aren't part of the repository from the list.
    for i in 0 1 2 3 4 5 6 7 8; do
        find "$1" -mindepth "$i" -maxdepth "$i" -iname \*.cddl -print0
    done
}

{
    find_cddl "$(dirname "$(dirname "$0")")"/spec | xargs --null cat
    find_cddl "$(dirname "$(dirname "$0")")"/attributes | xargs --null cat
} > "$1"
