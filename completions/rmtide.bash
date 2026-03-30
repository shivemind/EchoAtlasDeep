_rmtide() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="--mcp --version --help --config --theme"

    if [[ ${cur} == -* ]]; then
        COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
        return 0
    fi

    case "${prev}" in
        --theme)
            COMPREPLY=( $(compgen -W "catppuccin-mocha tokyonight gruvbox-dark one-dark" -- ${cur}) )
            return 0
            ;;
        --config)
            COMPREPLY=( $(compgen -f -- ${cur}) )
            return 0
            ;;
    esac

    COMPREPLY=( $(compgen -f -- ${cur}) )
}
complete -F _rmtide rmtide
