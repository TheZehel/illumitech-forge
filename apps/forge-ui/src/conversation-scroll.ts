import { useLayoutEffect, useRef, useState } from 'react';

/** Follow new output only while the reader stays near the end of this conversation. */
export function useConversationScroll(sessionId: string, revision: string) {
    const container = useRef<HTMLElement>(null);
    const following = useRef(true);
    const previousSession = useRef(sessionId);
    const [showLatest, setShowLatest] = useState(false);
    const jumpToLatest = () => {
        const node = container.current;
        if (node) node.scrollTop = node.scrollHeight;
        following.current = true;
        setShowLatest(false);
    };
    useLayoutEffect(() => {
        const changed = previousSession.current !== sessionId;
        previousSession.current = sessionId;
        if (changed || following.current) jumpToLatest();
        else setShowLatest(true);
    }, [sessionId, revision]);
    const onScroll = () => {
        const node = container.current;
        if (!node) return;
        following.current = node.scrollHeight - node.clientHeight - node.scrollTop < 96;
        setShowLatest(!following.current);
    };
    return { container, onScroll, jumpToLatest, showLatest };
}
