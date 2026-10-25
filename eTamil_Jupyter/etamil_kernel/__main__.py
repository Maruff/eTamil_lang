# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""`python -m etamil_kernel install` registers the kernel; Jupyter runs it with `-f <file>`."""

import argparse
import json
import sys
import tempfile
from pathlib import Path

KERNEL_NAME = "etamil"


def kernel_spec() -> dict:
    return {
        "argv": [sys.executable, "-m", "etamil_kernel", "-f", "{connection_file}"],
        "display_name": "eTamil",
        "language": "etamil",
        "metadata": {"debugger": False},
    }


def install(user: bool = True, prefix: "str | None" = None) -> str:
    from jupyter_client.kernelspec import KernelSpecManager

    with tempfile.TemporaryDirectory() as folder:
        (Path(folder) / "kernel.json").write_text(json.dumps(kernel_spec(), indent=2), encoding="utf-8")
        return KernelSpecManager().install_kernel_spec(folder, KERNEL_NAME, user=user and prefix is None, prefix=prefix)


def main(argv=None) -> None:
    argv = sys.argv[1:] if argv is None else argv
    if argv[:1] == ["install"]:
        parser = argparse.ArgumentParser(prog="python -m etamil_kernel install")
        where = parser.add_mutually_exclusive_group()
        where.add_argument("--user", action="store_true", help="for the current user (the default)")
        where.add_argument("--sys-prefix", action="store_true", help="inside the current Python environment")
        where.add_argument("--prefix", help="under this prefix")
        options = parser.parse_args(argv[1:])
        prefix = sys.prefix if options.sys_prefix else options.prefix
        print("installed the eTamil kernel at", install(user=prefix is None, prefix=prefix))
        return

    from ipykernel.kernelapp import IPKernelApp
    from .kernel import ETamilKernel

    IPKernelApp.launch_instance(argv=argv, kernel_class=ETamilKernel)


if __name__ == "__main__":
    main()
