#!/bin/bash

#set -x
set -e
set -u

CRASH_INVENTORY_FILE=crash_inventory.txt
CRASH_SUMMARY=crash_summary.txt
ARTIFACT_NAME=fuzz_results.tgz
INSPECT_TIMEOUT=20000
url_or_dir=$1
inspect_timeout=${2:-$INSPECT_TIMEOUT}
gh_run_id=
target=

function usage() {
    echo "$0 <run-id>"
    echo "Command speeds up processing fuzzing results."
    echo "The script gets Github run id or URL and checks fuzzing status."
    echo "It also tries to reproduce the crashes if this is the case."
    echo "This is to classify crashes and filter out duplicates."
    echo "  <run-id|run-url|local-afl-output-dir>  - Github action run id or url or AFL output directory"
    echo "  <inspect_timeout>                      - Abort inspection if it lasts longer than given timeout (default $INSPECT_TIMEOUT)"
    echo "                                           This is a workaround for 5min cancellation timeout, which occurs when inspecting"
    echo "                                           big number (+500) of crash files after the run was cancelled"
    exit
}

function validate_gh_run() {
    local view=
    view=$(gh run view $gh_run_id --json status,conclusion,url,workflowName,headBranch,headSha,displayTitle)
    status=$(jq -r '.status' <<< $view)
    conclusion=$(jq -r '.conclusion' <<< $view)
    url=$(jq -r '.url' <<< $view)
    name=$(jq -r '.workflowName' <<< $view)
    branch=$(jq -r '.headBranch' <<< $view)
    sha=$(jq -r '.headSha' <<< $view)

    title="$(jq -r '.displayTitle' <<< $view)"
    if [ $status = "in_progress" ] ; then
        echo "run $gh_run_id still in progress - come back later. Details: $url"
        exit 1
    fi
    if [ $conclusion = "failure" ] ; then
        echo "run $gh_run_id failed - nothing to process. Details: $url"
        exit 1
    fi
    if [ $name != "Fuzzing" ] ; then
        echo "run $gh_run_id is a '$name' not 'Fuzzing' run. Details: $url"
        exit 1
    fi

    # this variable is expected outside this function
    target=$(echo $title | grep 'target=' | sed -E 's/^.*target=([A-Za-z0-9_-]+) .*/\1/g')

    echo "Found run:"
    echo "  title : $title"
    echo "  branch: $branch"
    echo "  sha   : $sha"
}

function get_gh_artifacs() {
    echo "Seting up a work dir: $work_dir"
    mkdir -p $work_dir

    echo "Downloading $ARTIFACT_NAME"
    gh run download $gh_run_id -n $ARTIFACT_NAME -D $work_dir

    tar xf $work_dir/$ARTIFACT_NAME -C $work_dir || true
    rm $work_dir/$ARTIFACT_NAME
}

function show_gh_summary() {
    local d=${1:-}

    echo "url     : $url"
    cat $d/afl/summary | awk '/Summary stats/,/Time without/'
    echo "Fuzzing stats file: $d/afl/summary"
}

function show_crash_files() {
    local d=${1:-$afl_dir}
    echo "Crash/hang files:"
    find ${d}/*/* -type f ! -path */queue/* -name "id*" | xargs -I {} echo "    "{}
}

function inspect_crashes() {
    pushd $work_dir > /dev/null
    files=$(find ${afl_dir}/*/* -type f ! -path */queue/* -name "id*")

    if [ "$gh_run_id" != "" ] ; then
        show_gh_summary . > $CRASH_INVENTORY_FILE
    fi

    if [ "$files" != "" ] ; then
        echo "Inspecting found crashes"
        repo_dir=radixdlt-scrypto/fuzz-tests
        if [ "$gh_run_id" != "" ] ; then
            if [ ! -d radixdlt-scrypto ] ; then
                echo "Checking out the repository"
                git clone git@github.com:radixdlt/radixdlt-scrypto.git radixdlt-scrypto
            fi
            git -C radixdlt-scrypto checkout $sha
        else
            repo_path=$(cd ../.. ; pwd)
            ln -s $repo_path radixdlt-scrypto
        fi
        pushd $repo_dir > /dev/null
        echo "Building simple fuzzer"
        ./fuzz.sh simple build $target
        popd > /dev/null
        echo "Checking crash/hangs files"
        for f in $files ; do
            now=$(date +%s)
            elapsed=$(( now - started ))
            if [ $elapsed -gt $inspect_timeout ] ; then
                echo "Inspection timeout $inspect_timeout exceeded - interrupt" >> $CRASH_INVENTORY_FILE
                break
            fi

            # calling target directly to get rid of unnecessary debugs
            #./fuzz.sh simple run $target ../../$f >/dev/null || true
            cmd="${repo_dir}/target/release/${target} $f"
            echo
            echo "file    : $f"
            echo "command : $cmd"
            $cmd >output.log 2>&1 || true
            panic=$(grep panic output.log || true)
            echo "panic   : $panic"
            fname=$(echo $panic | sha256sum | awk '{print $1}').panic
            if [ ! -f $fname ] ; then
                echo -e "\npanic   : $panic" > $fname
            fi
            echo "file    : $f" >> $fname
        done

        cat <<EOF >> $CRASH_INVENTORY_FILE
Crash/hang info
command : radixdlt-scrypto/fuzz-tests/target/release/${target} <file>
EOF
        for f in *.panic ; do
            echo
            grep ^panic $f
            cnt=$(grep ^file $f | awk 'END{print NR}')
            echo "count   : $cnt"
            echo "list    : $f"
        done >> $CRASH_INVENTORY_FILE
        rm -f output.log
    else
        echo "No crashes found" >> $CRASH_INVENTORY_FILE
    fi

    popd > /dev/null

    ./process_crash_inventory.sh $work_dir/$CRASH_INVENTORY_FILE | tee $work_dir/$CRASH_SUMMARY

cat <<EOF

## Fuzzing crash summary
$(cat $work_dir/$CRASH_INVENTORY_FILE)

## Processing info
work dir  : $work_dir
inventory : $work_dir/$CRASH_INVENTORY_FILE
summary   : $work_dir/$CRASH_SUMMARY
EOF
    # copy crash summary to afl output dir, so it is packed to Github run artifact if running on Github
    if [ "$gh_run_id" = "" ] ; then
        cp $work_dir/$CRASH_INVENTORY_FILE $(dirname $afl_dir)
        cp $work_dir/$CRASH_SUMMARY $(dirname $afl_dir)
    fi
}

started=$(date +%s)

if [ $url_or_dir = "help" -o $url_or_dir = "h" ] ; then
    usage
fi
# check if argument is an existing AFL output directory
if [ -d $url_or_dir ] ; then
    if ls -A ${url_or_dir}/*/fuzzer_stats > /dev/null ; then
        work_dir=local_$(date -u  +%Y%m%d%H%M%S)
    else
        echo "This is not AFL output directory"
        exit 1
    fi
else
    gh_run_id=${url_or_dir##*/}
    work_dir=run_${gh_run_id}
fi


if [ "$gh_run_id" != "" ] ; then
    validate_gh_run
    # we know target after validate_gh_run
    afl_dir="afl/${target}"
    get_gh_artifacs
    show_gh_summary $work_dir
else
    target=${url_or_dir##*/}
    afl_dir=$url_or_dir
    mkdir -p $work_dir
    afl_path=$(cd $(dirname $afl_dir) ; pwd)
    ln -s $afl_path $work_dir
fi
show_crash_files $work_dir
inspect_crashes
