"use client";

import { useState } from "react";

export function useSelection<T extends { id: string }>() {
    const [selected, setSelected] = useState<Record<string, T>>({});

    const toggle = (item: T) => {
        setSelected((current) => {
            const next = { ...current };
            if (next[item.id]) {
                delete next[item.id];
            } else {
                next[item.id] = item;
            }
            return next;
        });
    };

    return {
        selectedIds: Object.keys(selected),
        selectedItems: Object.values(selected),
        toggle,
        clear: () => setSelected({}),
    };
}
