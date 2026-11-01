# Keeping the machine awake while agents work.

# $error is the system's text.
error-caffeine-hold = Could not keep the machine awake: { $error }
error-caffeine-no-working-agents = No agent is working, so there is nothing to stay awake for
error-caffeine-thread = Could not start the keep-awake thread
error-caffeine-worker-stopped = The keep-awake thread stopped; the display may sleep
# iPad keeps the display awake through UIKit, which only its main thread may change.
error-caffeine-main-thread = The display idle timer must be changed on the main thread
# Settings is the app's settings window; display keep-awake is one of its choices.
error-caffeine-unsupported-scope = Keeping only the system awake is unavailable on iPad; choose display keep-awake in Settings
