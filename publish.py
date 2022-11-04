#! /usr/bin/env python3
import os
import sys
import argparse
import json
import subprocess
from typing import List

bump_types = ["patch", "minor", "major"]


def get_version() -> str:
    metadata_process = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version=1"], capture_output=True)
    return json.loads(metadata_process.stdout)["packages"][0]["version"]


def update_version(version: str, type: str) -> str:
    tokens = version.split(".")
    major = int(tokens[0])
    minor = int(tokens[1])
    patch = int(tokens[2])

    if type == "major":
        major += 1
        minor = 0
        patch = 0
    elif type == "minor":
        minor += 1
        patch = 0
    else:
        patch += 1

    return "%d.%d.%d" % (major, minor, patch)


def check_tag_exists(tag: str) -> bool:
    tag_process = subprocess.run(
        ["git", "tag", "-l", tag], capture_output=True)
    return len(tag_process.stdout) > 0


def check_previous_commit_tagged() -> bool:
    return len(subprocess.run(["git", "tag", "--contains", "HEAD"], capture_output=True).stdout) > 0


def parse_git_output(output: bytes) -> List[str]:
    files = output.decode("utf-8").strip().split("\n")
    return [x.strip() for x in files]


def check_staged_changes() -> bool:
    return len(subprocess.run(["git", "diff", "--name-only", "--cached"], capture_output=True).stdout) > 0


# Main:
try:
    subprocess.check_output(['cargo', 'bump', '--version'])
except subprocess.CalledProcessError:
    print("[Error]\tFailed to run cargo bump. Try installing via 'cargo install cargo-bump'")
    sys.exit(1)

parser = argparse.ArgumentParser()

sub_parsers = parser.add_subparsers(dest="command")

bump_parser = sub_parsers.add_parser(name="bump")
undo_parser = sub_parsers.add_parser(name="undo")

bump_parser.add_argument("-t", "--type", choices=bump_types, default="",
                         help="Whether to bump patch, minor, or major versions.")
bump_parser.add_argument("-p", "--push", action="store_true",
                         help="If present, will automatically push to remote.")
bump_parser.add_argument("-d", "--dry", action='store_true',
                         help="If present, will perform a dry run with no changes.")

undo_parser.add_argument("-p", "--push", action="store_true",
                         help="If present, will automatically push to remote.")
undo_parser.add_argument("-d", "--dry", action='store_true',
                         help="If present, will perform a dry run with no changes.")

args = parser.parse_args(sys.argv[1:])

if args.command == "bump":
    bump_type = args.type

    if bump_type == "":
        bump_type = "patch"

    print("[Info]\tIncrementing %s..." % bump_type)

    # Check if previous commit was already tagged
    if check_previous_commit_tagged():
        print("[Warn]\tHEAD commit is already tagged. Seems like an attempt to bump with without any new changes.")
        ans = input(" > Would you like to proceed anyway? (y/n): ").lower()

        if ans == "n" or ans == "no":
            print(" > Aborting")
            exit(1)

    # Get versions
    current_version = get_version()
    next_version = update_version(current_version, bump_type)

    if check_tag_exists("v" + next_version):
        print("[Error]\tAttempting to bump from v%s to v%s, but the tag v%s already exists" % (
            current_version, next_version, next_version))

        if args.dry:
            exit(1)

        ans = input(
            " > Would you like to remove the tag and replace the tag? (y/n): ").lower()

        if ans == "y" or ans == "yes":
            subprocess.check_output(
                ["git", "tag", "-d", "origin", "v" + next_version])
        else:
            print(" > Aborting due to tag conflict")
            exit(1)

    if args.dry:
        print("[dry]\tBumped (%s) from v%s to v%s" %
              (bump_type, current_version, next_version))

        sys.exit(0)

    # Check for staged changes
    if check_staged_changes():
        print(
            "[Error]\tThere are staged changes. Please commit or unstage them before bumping.")
        exit(1)

    # Bump
    # os.environ["RUSTFLAGS"] = "-Awarnings" # Suppresses warning, but too slow

    subprocess.check_output(["cargo", "bump", bump_type])
    subprocess.check_output(["cargo", "check", "--quiet"])

    # Commit
    new_version = get_version()
    assert(new_version == next_version)

    message = "Bumped version from v%s to v%s" % (current_version, new_version)

    subprocess.check_output(["git", "add", "Cargo.toml", "Cargo.lock"])
    subprocess.check_output(["git", "commit", "-m", message])
    subprocess.check_output(["git", "tag", "v%s" % (new_version)])

    if not args.push:
        ans = input("Do you want to push your changes? (y/n): ")

        if ans.lower() in ["y", "yes"]:
            args.push = True

    if args.push:
        print("[Info]\tPulling to check... ", end="")

        pull_process = subprocess.run(
            ["git", "pull"], capture_output=True)

        if "Already up to date." in str(pull_process.stdout):
            print("ok.")
        else:
            print("warning.")
            print(
                "Remote contains changes not present in local. Make sure remote is update before bumping")
            exit(1)

        print("[Info]\tPushing to origin...")
        subprocess.check_output(["git", "push", "origin", "main", "--tags"])
    else:
        print("[Info]\tApplied changes locally. Run 'git push origin main --tags' to push tags to remote and trigger a workflow.")


elif args.command == "undo":
    if check_previous_commit_tagged():
        # Check previous commit
        for file in parse_git_output(subprocess.run(["git", "diff", "--name-only", "HEAD", "HEAD~1"], capture_output=True).stdout):
            if not file in ["Cargo.toml", "Cargo.lock"]:
                print("[Error]\tUnable to undo; previous commit is a bump but contains other changes. Please reset and remove the tag manually")
                exit(1)

        current_version = get_version()

        subprocess.check_output(["git", "reset", "--mixed", "HEAD~1"])
        subprocess.check_output(
            ["git", "stash", "push", "Cargo.lock", "Cargo.toml"])

        subprocess.check_output(["git", "tag", "-d", "v" + current_version])

        reverted_version = get_version()

        if args.push:
            subprocess.check_output(
                ["git", "push", "-d", "origin", "v" + current_version])

            print(
                "[Info]\tRemove tags will only remove tags and revert commits locally.")
            ans = input(
                " > Do you want to also push the reverted commits? (WARNING! THIS WILL PERFORM A FORCE PUSH) (y/n): ").lower()

            if ans == "y" or ans == "yes":
                subprocess.check_output(
                    ["git", "push", "-f", "origin", "main"])

        print("[Info]\tReverted from v%s to v%s." %
              (current_version, reverted_version))
        exit(0)
    else:
        print("[Error]\tUnable to undo. Previous commit isn't tagged.")

        exit(1)

elif args.command is None:
    parser.print_help()
    exit(0)
else:
    print("[Error]\tUnknown command: %s" % args.command)
    exit(1)
