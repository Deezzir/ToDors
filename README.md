# ToDors

A terminal based Todo list manager written in Rust. My first Rust project.

![img](demo.png)

## Quick Start

```bash
cargo run --release
```

## Controls

| Key                                                      | Descritption                         |
|----------------------------------------------------------|--------------------------------------|
| <kbd>k/↑</kbd>,<kbd>j/↓</kbd>                            | Move UP/DOWN                         |
| <kbd>SHIFT+k/SHIFT+↑</kbd>,<kbd>SHIFT+j/SHIFT+↓</kbd>    | Drag item UP/DOWN                    |
| <kbd>g</kbd>,<kbd>G</kbd>,<kbd>h</kbd>                   | Jump to START/END/HALF of the list   |
| <kbd>d</kbd>                                             | Delete 'Done' item/subtask           |
| <kbd>i</kbd>                                             | Insert a new 'Todo' item             |
| <kbd>a</kbd>                                             | Add subtask to current 'Todo' item   |
| <kbd>u</kbd>                                             | Undo last action                     |
| <kbd>r</kbd>                                             | Edit current item                    |
| <kbd>t</kbd>                                             | Hide subtasks                        |
| <kbd>?</kbd>                                             | Show help                            |
| <kbd>ENTER</kbd>                                         | Mark element/Save edited item        |
| <kbd>TAB</kbd>                                           | Switch between 'Todos'/'Dones'       |
| <kbd>ESC</kbd>                                           | Cancel editing/inserting             |
| <kbd>q</kbd>,<kbd>CTRL+c</kbd>                           | Quit                                 |
