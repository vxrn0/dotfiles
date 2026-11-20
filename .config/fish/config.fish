if status is-interactive
    set fish_greeting

    function starship_transient_prompt_func
        starship module character
    end
    if test "$TERM" != "linux"
        starship init fish | source
        enable_transience
    end

    if test -f ~/.local/state/quickshell/user/generated/terminal/sequences.txt
        cat ~/.local/state/quickshell/user/generated/terminal/sequences.txt
    end

    alias clear "printf '\033[2J\033[3J\033[1;1H'"
    alias celar "printf '\033[2J\033[3J\033[1;1H'"
    alias claer "printf '\033[2J\033[3J\033[1;1H'"
    alias pamcan pacman
    alias q 'qs -c dotfiles'
    if test "$TERM" != "linux"
        alias ls 'eza --icons=auto'

        fish_add_path $HOME/.local/bin
        if test -d $HOME/.local/node/bin
            fish_add_path $HOME/.local/node/bin
        end
    end
end
