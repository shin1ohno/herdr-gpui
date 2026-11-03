# iPad window controls

Native screenshots from the iPad Pro 11-inch (4th generation) simulator on
iOS 26.5, landscape, with a 375 × 834 pt app window. All terminal content is
from an isolated QA fixture. The full screen is retained around the app window.

Before: `ea7bda1c`. The system window controls cover the Menu target, and the
compact bar SVGs have no paint color. After: the bar uses UIKit’s physical
corner-adapted insets and gives its SVGs an explicit theme color. The terminal
keeps its full width. The Latte after image follows a full-width-to-compact resize.

| Theme | Before | After |
| --- | --- | --- |
| Nord | [Native screen](before-nord.png) | [Native screen](after-nord.png) |
| Catppuccin Latte | [Native screen](before-latte.png) | [Native screen](after-latte.png) |

[Menu opened by tapping the cleared target after resize](menu-open-latte.png).
The same target was also tapped to close the sidebar.
