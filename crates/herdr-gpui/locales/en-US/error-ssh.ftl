# iPadOS only: why this iPad's own SSH connection to a computer failed. The
# device key is the key this iPad signs SSH logins with.

error-ssh-auth = the host refused this device's key
error-ssh-cancelled = cancelled
error-ssh-host-key-changed = the host key changed
error-ssh-host-key-unknown = the host key is not trusted yet
error-ssh-protocol = the SSH connection failed
error-ssh-timeout = the host did not answer in time
error-ssh-unreachable = the host is unreachable

# $reason is herdr-ssh's own description of what is wrong.
error-ssh-invalid-client-key = invalid device key: { $reason }
# $reason is herdr-ssh's own description of what is wrong.
error-ssh-invalid-target = invalid SSH target: { $reason }
# $operation is herdr-ssh's own description of what went wrong with the file,
# such as "is too large".
error-ssh-known-hosts = known hosts file { $operation }
error-ssh-pinned-meanwhile = the host's trusted keys changed before confirmation
