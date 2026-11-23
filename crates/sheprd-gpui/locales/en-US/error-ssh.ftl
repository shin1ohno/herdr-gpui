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

# A forward carries connections from this iPad to a port on the computer's
# own loopback over its own SSH connection.
error-ssh-forward-busy = too many forwarded connections at once
# $failure is one of the error-ssh-* connection failures above.
error-ssh-forward-connect = the forwarding connection failed: { $failure }
# $failure is one of the error-ssh-* connection failures above.
error-ssh-forward-connection-lost = the forwarding connection was lost: { $failure }
error-ssh-forward-invalid-port = invalid port to forward to
error-ssh-forward-not-loopback = only the host's loopback can be forwarded to
error-ssh-forward-port-closed = nothing is listening on that port on the host
error-ssh-forward-prohibited = the host does not allow forwarding for this key
error-ssh-forward-refused = the host refused the forwarded connection
error-ssh-forward-stopped = the forward was stopped
error-ssh-forward-timeout = the host did not answer the forwarded connection in time
