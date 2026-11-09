# walked
Terminal file manager that looks like an interactive [`ls`](https://en.wikipedia.org/wiki/Ls). (Inspired by [`Dired`](https://www.deinprogramm.de/sperber/software/dired/))

![walked demo gif](./github/walked-demo.gif)

# Features
 - Navigate through the filesystem, create, remove, rename, copy, and paste files/directories.
 - Vim motions for navigation. Seperate INSERT (i), NORMAL (Esc), and SELECT (v) modes for different operations.
 - Incremental search.
 - Seamlessly unzip/untar files and 'walk' into them like folders.
 - Includes a command prompt with tab-completions accessible via ':' in NORMAL mode.
   * This command prompt - in addition to other commands - includes the command :sh which lets you run arbitrary shell commands on selected files. For instance, `:sh mv % %.bak` would add a '.bak' suffix to all selected files.
 - [`tmux`](https://tmux.app/) integration for commands run with :sh. If the `TMUX` environment variable is available, commands are run inside a configurable `tmux` popup. :sh! can be used to explicitly run the command without touching `tmux`.
 - Highly configurable (See `Configuration` section below).

Work in progress/planned features:
 - Fuzzy search
 - Exhaustive support for symlinks

# Build Instructions
```console
  $ git clone https://github.com/serd223/walked.git
  $ cd walked
  $ cargo install --path .
```

For you to be able to change your directory upon quitting `walked`, you will need to add something along the lines of the following snippet to your autoexec script (.zshrc, .bashrc, Powershell_profile.ps1, etc):
```bash
alias wd="cd \$(walked path/to/your/walked.toml)"
```

Now, you can use the `wd` command to use `walked` and change your working directory with it.

# Configuration
`walked` uses a simple `TOML` file for configuration. You can optionally supply the path to your configuration file as a command line argument to the program. (This would typically be done in your autoexec script mentioned above)
## Example
```console
  $ walked myconf.toml # uses default configuration if myconf.toml couldn't be found
```
## Writing your configuration file
For a complete list of configurable options, see `Config` in [config.rs](https://github.com/serd223/walked/blob/master/src/config.rs).

If an option isn't specified inside your configuration file, the default value will be used.

Boolean and string options can be written like normal. For example:
```toml
# my_conf.toml
normal_mode_text = "NORM"
shell_command_placeholder = "%" # Expanded to the current selected file when using :sh
tmux_display_popup_command = "display-popup -E -w 90% -h 90%"
```
Keybindings can be described with a very simple string format like this:
```toml
new_file = "C-N" # Control + N, typing a lowercase 'n' wouldn't work in this case
quit = "cA-Esc" # Control + Alt + Escape
# Both control and alt modifiers can be uppercase or lowercase, so this would be valid too:
# quit = "Ca-Esc" # Control + Alt + Escape
# or this:
# quit = "ca-Esc" # Control + Alt + Escape
# and so on

# Space can be written as-is
dir_walk = " "
# It can be used with any modifier just like other characters
dir_up = "S- " # Shift + Space

# Note that Control + Shift style bindings like this:
copy = "CS-c" # Control + Shift + C
# Only work on Windows due to the Console API there. On any other systems using standard
# terminal protocols, terminals can't distinguish between Control and Control + Shift modifiers.

# Please note that characters that have uppercase/lowercase variants may not work properly with the
# shift modifier as the shift key is commonly used to reverse the capitalization of the typed letter

# Here is the list of supported 'special' keys: (these are case sensitive, so you can't write "up" in your config)
# Backspace
# Enter
# Left
# Right
# Up
# Down
# Home
# End
# PageUp
# PageDown
# Tab
# BackTab
# Delete
# Insert
# Esc
# Function keys can be written normally (F6, F9, F11, etc)

```

## Default Keybindings
`new_file`: C-n

`new_directory`: C-b

`duplicate`: C-d

`remove`: C-r

`copy`: C-c

`paste`: C-v

`incremental_search`: /

`next_search_result`: n

`prev_search_result`: N

`up`: k

`down`: j

`left`: h

`column_left`: H

`right`: l

`column_right`: L

`insert_mode`: i

`normal_mode`: `Esc`

`select_mode`: v

`command_mode`: :

`quit`: q

`dir_walk`: `Space`

`dir_up`: x
