host-key-code-settings = Show this computer's host key
host-key-code-settings-description = Show a QR code of this computer's SSH host-key fingerprints for adding it on an iPad.
host-key-code-settings-button = Show code

host-key-code-title = This computer's SSH host key
host-key-code-intro = Scan this code with your iPad's Camera app, or copy it into Add Device. Confirm that the code came from this computer's own screen.
host-key-code-loading = Reading SSH host public keys…
host-key-code-computer = Computer: { $host }
host-key-code-public-only = This code contains only public-key fingerprints. It grants no access. The computer still needs Remote Login or OpenSSH enabled.
host-key-code-copy = Copy code
host-key-code-close = Close

host-key-code-paste-placeholder = herdr-ipad://host-key?…
host-key-code-paste-label = Paste the host-key code shown on the computer (optional)
host-key-code-use-pasted = Use pasted code
host-key-code-confirm-warning = Only accept a code you just scanned from this computer’s own screen. Any website or app can send this link. Enter the SSH target above, then compare every fingerprint before accepting.
host-key-code-accept = Accept this computer’s code
host-key-code-dismiss = Dismiss code
host-key-code-ready = This code applies only to this Add Device attempt and this SSH target.
host-key-code-clear = Clear code
host-key-code-enter-target = Enter the computer’s SSH target before accepting its code.
host-key-code-host-hint = Computer name hint: { $host }
host-key-code-matched = The SSH host key matched the computer’s code.
host-key-code-mismatch = The SSH host key of { $target } does not match the computer’s code. No key was trusted. Expected: { $expected } Offered: { $offered }
host-key-code-already-pinned = { $target } already has a pinned key. A computer’s code never replaces an existing pin.

host-key-code-error-fingerprints = The host-key code must contain distinct supported SSH key types and SHA-256 fingerprints
host-key-code-error-host-hint = The host name hint in the host-key code is invalid
host-key-code-error-invalid = The host-key code is not a valid herdr-ipad host-key URL
host-key-code-error-no-host-keys = No SSH host public keys were found; enable Remote Login or install and configure OpenSSH on this computer
host-key-code-error-public-key = The SSH host public key at { $path } is invalid
host-key-code-error-read = Could not read the SSH host public key at { $path }: { $error }
host-key-code-error-too-large = The host-key code is too large
host-key-code-error-unsupported = SSH host public keys are unavailable on this platform
host-key-code-error-version = The host-key code version is not supported
