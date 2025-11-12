/**
 * Decorative arrow component that appears above view cards.
 * Creates a visual connection between the popover window and the tray icon.
 * 
 * The arrow direction adapts based on popover position:
 * - "up": Popover is below tray (arrow points up to tray above)
 * - "down": Popover is above tray (arrow points down to tray below)
 */
export interface ArrowProps {
    /** Arrow direction: "up" (points up) or "down" (points down) */
    direction?: "up" | "down";
}

export function Arrow({ direction = "up" }: ArrowProps) {
    const isUp = direction === "up";
    
    return (
        <div className="w-full h-[14px] relative -mb-px">
            {isUp ? (
                // Arrow pointing UP (popover below tray)
                <>
                    {/* Outer arrow (border) */}
                    <div className="absolute top-0 left-1/2 -translate-x-1/2 w-0 h-0 border-l-[9px] border-l-transparent border-r-[9px] border-r-transparent border-b-[14px] border-b-border" />
                    {/* Inner arrow (fill) */}
                    <div className="absolute top-px left-1/2 -translate-x-1/2 w-0 h-0 border-l-[8px] border-l-transparent border-r-[8px] border-r-transparent border-b-[13px] border-b-bgCard" />
                </>
            ) : (
                // Arrow pointing DOWN (popover above tray)
                <>
                    {/* Outer arrow (border) */}
                    <div className="absolute bottom-0 left-1/2 -translate-x-1/2 w-0 h-0 border-l-[9px] border-l-transparent border-r-[9px] border-r-transparent border-t-[14px] border-t-border" />
                    {/* Inner arrow (fill) */}
                    <div className="absolute bottom-px left-1/2 -translate-x-1/2 w-0 h-0 border-l-[8px] border-l-transparent border-r-[8px] border-r-transparent border-t-[13px] border-t-bgCard" />
                </>
            )}
        </div>
    );
}

