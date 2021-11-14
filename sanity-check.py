#!/usr/bin/python

import os
import sys

LICENSE_DELIMIT = "///////////////////////////////////////////////////////////////////////////////L"

LICENSE = """// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details"""

problems = {}

# validate license headers
for root, dirs, files in os.walk("kernel"):
    for file in files:
        if file.endswith(".rs"):
            text = ""
            with open(os.path.join(root, file), "rt") as f:
                text = f.read()
            if not text.startswith("{}\n{}\n{}".format(LICENSE_DELIMIT, LICENSE, LICENSE_DELIMIT)):
                pathkey = os.path.join(root, file).replace("\\", "/")
                # try auto update
                split = text.split(LICENSE_DELIMIT)
                if len(split) == 3:
                    with open(os.path.join(root, file), "wt") as f:
                        f.write("{}\n{}\n{}{}".format(LICENSE_DELIMIT, LICENSE, LICENSE_DELIMIT, split[2]))
                    print("Updated license header in {}".format(pathkey))
                else:
                    if pathkey not in problems:
                        problems[pathkey] = []
                    problems[pathkey].append("Needs license header fixed")

if len(problems) == 0:
    print("No issues found.")
    sys.exit(0)

print("Issues:")
longestName = 0
for k in problems:
    longestName = max(longestName, len(k))

for k, v in problems.items():
    for p in v:
        print("{} | {}".format(k.ljust(longestName), p))
