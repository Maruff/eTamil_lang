#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// The eTamil language server, for editors. It speaks the Language Server
// Protocol on stdin and stdout, so an editor runs it; you do not.
"use strict";

require("../scripts/launch.js").launch("etamil-lsp");
