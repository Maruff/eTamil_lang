{
  "targets": [
    {
      "target_name": "tree_sitter_etamil_binding",
      "dependencies": [
        "<!(node -p \"require('node-addon-api').targets\"):node_addon_api_except",
      ],
      "include_dirs": [
        "src",
      ],
      "sources": [
        "bindings/node/binding.cc",
        "src/parser.c",
      ],
      "variables": {
        "has_scanner": "<!(node -p \"fs.existsSync('src/scanner.c')\")"
      },
      "conditions": [
        ["has_scanner=='true'", {
          "sources+": ["src/scanner.c"],
        }],
        ["OS!='win'", {
          "cflags_c": [
            "-std=c11",
          ],
        }, { # OS == "win"
          "cflags_c": [
            "/std:c11",
            "/utf-8",
          ],
          # node-gyp ignores cflags on Windows; MSVC options go here. /utf-8 is not
          # optional for this grammar: its keywords are Tamil, and without it MSVC
          # reads parser.c in the ANSI code page and turns every non-ASCII node name
          # into '?', so a query that names a keyword node no longer compiles.
          "msvs_settings": {
            "VCCLCompilerTool": {
              "AdditionalOptions": ["/utf-8"],
            },
          },
        }],
      ],
    }
  ]
}
