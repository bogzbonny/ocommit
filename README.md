# ocommit

`ocommit` quickly create a git commit message with ollama and commit,
**locally**.

## Workflow

0. `ocommit`
1. Shows you the current `git status` and generates a commit message based off
   the diff 
2. press [Enter] to accept the message, [Tab]/['r'] to regenerate (or other keys to
   escape).
3. `ocommit` stages the changes and commits them.

The tool also supports dry‑runing (`-d` / `--dry`) which doesn't perform actual
staging or commits

## Installation

Install [rust](https://rust-lang.org/tools/install/).

```bash
cargo install ocommit
```

Make sure you have [ollama](https://ollama.com/) installed and pull whatever
model you want to use. The default is qwen2.5-coder:3b. 

```
ollama pull qwen2.5-coder:3b
```

## Configuration

The program looks for a YAML configuration file at `$HOME/.config/ocommit.yaml`
 - it creates it with default values if it doesn't exist.

```yaml
ollama_model: qwen2.5-coder:3b   # required – name of the model to use. default: qwen2.5-coder:3b
ignore_files:                    # names of files/folders to ignore (in addition
- Cargo.lock                     #   to what's already in the .gitignore)
```

## Facts

 - 1 file (main.rs) with < 250 LOC (EZ to audit) 
 - for speed, if the diff is > ~1000 LOC then only file name changes will be sent to
   ollama
 - llm structured outputs are used to remove llm niceties ("sure I can write you
   a commit message..." lol why)

## Usage

```bash
ocommit          # run
ocommit --dry    # show the generated commit message without committing
ocommit -d       # short alias for `--dry`
```

### Contributing

yoo!
