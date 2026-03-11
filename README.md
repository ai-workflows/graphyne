# graphyne

Graphyne is a Rust implementation of a functional data-flow virtual machine and JSON-based intermediate representation.

## Project layout

- `src/main.rs` — CLI entrypoint with `await` and `stream` subcommands
- `src/api.rs` — high-level API for loading, binding, and executing programs
- `src/binder/` — deserializes the JSON intermediate format and binds collections into runtime state
- `src/runtime/data/` — stored/live runtime value model, functions, opcodes, and type definitions
- `src/runtime/static_state/` — static symbol storage for functions, constants, imports, and types
- `src/runtime/vm/` — call orchestration, operator dispatch, output routing, and execution management
- `examples/intermediate/` — sample compiled programs for manual testing

## Build

```bash
. $HOME/.cargo/env
cargo build
```

## Test

```bash
. $HOME/.cargo/env
cargo test
```

## Run

The CLI expects a JSON intermediate file and executes the `main` function inside it.

### Await mode

Wait for all outputs before printing them:

```bash
. $HOME/.cargo/env
cargo run -- await -i examples/intermediate/test_compiled.json
```

You can also run the nested collection example:

```bash
cargo run -- await -i examples/intermediate/double_list.json
```

And an object/custom-type example:

```bash
cargo run -- await -i examples/intermediate/object_person.json
```

A composition example that combines imports, custom types, object field access, and arithmetic:

```bash
cargo run -- await -i examples/intermediate/imported_object_sum.json
```

### Stream mode

Print each output as soon as it becomes available:

```bash
. $HOME/.cargo/env
cargo run -- stream -i examples/intermediate/test_compiled.json
```

### Worker count

Optionally control the Rayon worker pool size:

```bash
cargo run -- await -i examples/intermediate/test_compiled.json --workers 4
```

### Verbose mode

Use `--verbose` to print execution metadata such as mode, input file, and worker count before results are emitted:

```bash
cargo run -- stream -i examples/intermediate/test_compiled.json --verbose
```

### Output streams and exit codes

- program results are written to `stdout`
- verbose diagnostics and errors are written to `stderr`
- the CLI exits with status code `0` on success
- the CLI exits with a non-zero status code for load, bind, startup, or runtime failures

This makes it suitable for shell scripting and redirection:

```bash
cargo run -- await -i examples/intermediate/test_compiled.json >results.txt 2>diagnostics.txt
```

## Intermediate format overview

At a high level, a program is a `Collection` containing:

- `functions`
- `constants`
- `collections` (nested collections)
- `types`
- `imports`

Each function contains a graph with:

- `values` — named function-local values, optionally with constants
- `ops` — operations referencing input and output value symbols
- `input_vals` — formal arguments
- `output_vals` — return values

A minimal example:

```json
{
  "functions": {
    "main": {
      "graph": {
        "values": [["lhs", 2], ["rhs", 3], "sum"],
        "ops": [["Add", ["lhs", "rhs"], ["sum"]]],
        "input_vals": [],
        "output_vals": ["sum"]
      }
    }
  }
}
```

A richer object/type example using the now-supported map-form type syntax:

```json
{
  "types": {
    "Person": {
      "name": "str",
      "age": "int"
    }
  },
  "functions": {
    "main": {
      "graph": {
        "values": [
          "Person",
          ["name", "Ada"],
          ["age", 36],
          "person",
          ["age_key", "age"],
          ["new_age", 37],
          "updated_person",
          "updated_person_dict",
          "updated_age"
        ],
        "ops": [
          ["Static", ["Person"], ["Person"]],
          ["Init", ["Person", "name", "age"], ["person"]],
          ["Set", ["person", "age_key", "new_age"], ["updated_person"]],
          ["AsDictionary", ["updated_person"], ["updated_person_dict"]],
          ["Get", ["updated_person_dict", "age_key"], ["updated_age"]]
        ],
        "input_vals": [],
        "output_vals": ["updated_person", "updated_age"]
      }
    }
  }
}
```

You can run the shipped version directly:

```bash
cargo run -- await -i examples/intermediate/object_person.json
```

Import paths may be written in either of these forms:

- root-relative: `["lib", "double"]`
- root-symbol-prefixed: `["root", "lib", "double"]`

Both are accepted when the bound program symbol is `root`.

## Execution model

1. The CLI or API loads a JSON collection.
2. The binder allocates static references and lowers functions/constants/types into runtime structures.
3. The VM initializes a call context for `main`.
4. Operations run when their input values become available.
5. Outputs are either streamed through a channel or collected and returned in order.

## Notes for contributors

- `cargo test` is currently the best quick validation target.
- Example programs under `examples/intermediate/` are useful for smoke testing.
- `todo.md` contains future language and runtime ideas that are not yet implemented.
