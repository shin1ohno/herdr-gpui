ipad-sidebar-tabs = { $count ->
    [one] 1 tab
   *[other] { $count } tabs
    }
ipad-sidebar-needs-you = Needs you
ipad-sidebar-worktree = worktree
ipad-sidebar-new-space = New space
ipad-sidebar-settings = Settings
ipad-sidebar-host-origin = { $origin ->
    [ssh] SSH
    [tailscale] Tailscale
    [wsl] WSL
    [cloud] Cloud
   *[local] This iPad
    }
ipad-sidebar-host-generation = Herdr { $generation }
