# walkEd
Terminal file manager inspired by [Emacs](https://www.gnu.org/software/emacs/) [Dired](https://www.gnu.org/software/emacs/manual/html_node/emacs/Dired.html) as a concept. `walkEd` doesn't try to recreate Dired. I was just inspired by the general style of Dired from what I've seen (never used it).

# Features
`walkEd` is pretty simple, it can help you navigate through the filesystem and it can duplicate, copy, paste, create, remove and rename files and directories.

# Build Instructions
```console
  $ git clone https://github.com/serd223/walked.git
  $ cd walked
  $ cargo install --path .
```

For you to be able to change your directory upon quitting `walkEd`, you will need to add something along the lines of the following script to your autoexec script (.bashrc, Powershell_profile.ps1, etc):
```powershell
# Example Powershell profile
function wd() {
  # I am not sure why the 3>&1 is necessary, as it just redirects the `Warning` output
  # stream to the `Success` output stream but the script doesn't seem to work without it.
  cd $(walked.exe 3>&1)
}
```

Now, you can use the `wd` command to use `walkEd` and change your working directory with it.

# Keybinds

## Configuration
By default, `walkEd` doesn't have a configuration file. The path to your desired configuration file can be supplied to the program directly as a command line arguement and if the file doesn't exist, `walkEd` will create the file in the desired path and fill its contents with the default configuration. It is recommended to first run `walkEd` with your desired configuration file path without creating the file, so later you can edit the generated default configuration easily.
### Example
```console
  $ walked myconf.toml
  /walked/working/directory
  $ vim myconf.toml
  # edit your config file..
  $ walked myconf.toml
  # now walked will be using your desired configuration
```

## Default Keybindings
`new_file`: Ctrl-n

`new_directory`: Ctrl-m

`duplicate`: Ctrl-d

`remove`: Ctrl-x

`copy`: Ctrl-y

`paste`: Ctrl-p

`up`: k

`down`: j

`left`: h

`right`: l

`insert_mode`: i

`normal_mode`: `Escape`

`quit`: q

`dir_walk`: `Space`

`dir_up`: x
